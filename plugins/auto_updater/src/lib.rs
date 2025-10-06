//! Auto Updater Plugin for Iced
//!
//! This plugin automatically checks for updates from GitHub releases,
//! downloads them, verifies SHA256 checksums, and installs them.
//!
//! # Features
//!
//! - Check for updates from GitHub releases
//! - Automatic OS and architecture detection
//! - Download release assets
//! - Verify SHA256 checksums
//! - Install macOS bundles (.dmg, .tar.gz, .zip)
//! - Install Linux packages (.deb for Debian/Ubuntu)
//! - Progress tracking for downloads
//! - Automatic or manual update checks
//!
//! # Example
//!
//! ```ignore
//! use iced_auto_updater_plugin::{AutoUpdaterPlugin, UpdaterConfig};
//!
//! const APP_NAME: &str = "my_app";
//!
//! fn main() -> iced::Result {
//!     let mut plugins = PluginManager::new();
//!
//!     let config = UpdaterConfig::new("owner", "repo", env!("CARGO_PKG_VERSION"));
//!     let updater_handle = plugins.install(AutoUpdaterPlugin::new(APP_NAME.to_string(), config));
//!
//!     // Check for updates manually
//!     let task = updater_handle.dispatch(AutoUpdaterMessage::CheckForUpdates);
//!
//!     iced::application(App::new, App::update, App::view)
//!         .run()
//! }
//! ```

use iced::time::every;
use iced::{Subscription, Task};
use iced_plugins::Plugin;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;
use tokio::fs;
use tokio::io::AsyncWriteExt;

/// Configuration for the auto updater
#[derive(Debug, Clone)]
pub struct UpdaterConfig {
    /// GitHub repository owner
    pub owner: String,
    /// GitHub repository name
    pub repo: String,
    /// Current version of the application
    pub current_version: String,
    /// Auto-check interval in seconds (0 = disabled)
    pub auto_check_interval: u64,
}

impl UpdaterConfig {
    /// Create a new updater config
    pub fn new(
        owner: impl Into<String>,
        repo: impl Into<String>,
        current_version: impl Into<String>,
    ) -> Self {
        Self {
            owner: owner.into(),
            repo: repo.into(),
            current_version: current_version.into(),
            auto_check_interval: 0, // Disabled by default
        }
    }

    /// Enable automatic update checking with specified interval
    pub fn with_auto_check(mut self, interval_secs: u64) -> Self {
        self.auto_check_interval = interval_secs;
        self
    }
}

/// GitHub release information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseInfo {
    pub tag_name: String,
    pub name: String,
    pub body: Option<String>,
    pub html_url: String,
    pub assets: Vec<ReleaseAsset>,
}

/// GitHub release asset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseAsset {
    pub name: String,
    pub browser_download_url: String,
    pub size: u64,
}

/// Download progress information
#[derive(Debug, Clone)]
pub struct DownloadProgress {
    pub downloaded: u64,
    pub total: u64,
}

impl DownloadProgress {
    pub fn percentage(&self) -> f32 {
        if self.total == 0 {
            0.0
        } else {
            (self.downloaded as f32 / self.total as f32) * 100.0
        }
    }
}

/// Messages that the auto updater plugin handles
#[derive(Clone, Debug)]
pub enum AutoUpdaterMessage {
    /// Check for updates from GitHub
    CheckForUpdates,
    /// Update check completed
    UpdateCheckResult(Result<Option<ReleaseInfo>, String>),
    /// Download and install update
    DownloadAndInstall(ReleaseInfo),
    /// Download progress update
    DownloadProgress(DownloadProgress),
    /// Download completed
    DownloadCompleted(Result<PathBuf, String>),
    /// SHA256 verification result
    VerificationResult(Result<PathBuf, String>),
    /// Installation result
    InstallationResult(Result<(), String>),
    /// Auto-check timer tick
    AutoCheckTick,
}

/// Output messages emitted by the auto updater plugin
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum AutoUpdaterOutput {
    /// Update is available
    UpdateAvailable(ReleaseInfo),
    /// No update available
    NoUpdateAvailable,
    /// Download started
    DownloadStarted(ReleaseInfo),
    /// Download progress
    DownloadProgress(DownloadProgress),
    /// Download completed
    DownloadCompleted(PathBuf),
    /// Verification succeeded
    VerificationSucceeded(PathBuf),
    /// Verification failed
    VerificationFailed(String),
    /// Installation started
    InstallationStarted,
    /// Installation completed successfully
    InstallationCompleted,
    /// An error occurred
    Error(String),
}

/// The plugin state held by the PluginManager
pub struct AutoUpdaterState {
    /// Current download progress
    pub download_progress: Option<DownloadProgress>,
    /// Latest release info
    pub latest_release: Option<ReleaseInfo>,
    /// Whether an update is being processed
    pub is_updating: bool,
    /// Downloaded file path
    pub downloaded_file: Option<PathBuf>,
}

/// Auto updater plugin
pub struct AutoUpdaterPlugin {
    app_name: String,
    config: UpdaterConfig,
}

impl AutoUpdaterPlugin {
    /// Create a new auto updater plugin
    pub fn new(app_name: String, config: UpdaterConfig) -> Self {
        Self { app_name, config }
    }

    /// Get the download directory
    fn download_dir(&self) -> PathBuf {
        directories::BaseDirs::new()
            .map(|dirs| dirs.cache_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."))
            .join(&self.app_name)
            .join("updates")
    }

    /// Check for updates from GitHub
    async fn check_for_updates(
        owner: String,
        repo: String,
        current_version: String,
    ) -> Result<Option<ReleaseInfo>, String> {
        let url = format!(
            "https://api.github.com/repos/{}/{}/releases/latest",
            owner, repo
        );

        let client = reqwest::Client::builder()
            .user_agent("iced-auto-updater")
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

        let response = client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Failed to fetch release info: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("GitHub API returned status: {}", response.status()));
        }

        let release: ReleaseInfo = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse release info: {}", e))?;

        // Simple version comparison (remove 'v' prefix if present)
        let latest_version = release.tag_name.trim_start_matches('v');
        let current = current_version.trim_start_matches('v');

        if latest_version != current {
            Ok(Some(release))
        } else {
            Ok(None)
        }
    }

    /// Download a file with progress tracking
    async fn download_file(url: String, dest_path: PathBuf) -> Result<PathBuf, String> {
        let client = reqwest::Client::new();
        let response = client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Failed to download: {}", e))?;

        if !response.status().is_success() {
            return Err(format!(
                "Download failed with status: {}",
                response.status()
            ));
        }

        // Create parent directory if it doesn't exist
        if let Some(parent) = dest_path.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|e| format!("Failed to create download directory: {}", e))?;
        }

        let bytes = response
            .bytes()
            .await
            .map_err(|e| format!("Failed to read response: {}", e))?;

        let mut file = fs::File::create(&dest_path)
            .await
            .map_err(|e| format!("Failed to create file: {}", e))?;

        file.write_all(&bytes)
            .await
            .map_err(|e| format!("Failed to write file: {}", e))?;

        Ok(dest_path)
    }

    /// Download SHA256 checksum file
    async fn download_sha256(url: String) -> Result<String, String> {
        let client = reqwest::Client::new();
        let response = client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Failed to download SHA256: {}", e))?;

        if !response.status().is_success() {
            return Err(format!(
                "SHA256 download failed with status: {}",
                response.status()
            ));
        }

        let content = response
            .text()
            .await
            .map_err(|e| format!("Failed to read SHA256: {}", e))?;

        // Extract just the hash (first 64 characters)
        let hash = content
            .split_whitespace()
            .next()
            .ok_or_else(|| "Invalid SHA256 format".to_string())?
            .to_string();

        Ok(hash)
    }

    /// Verify SHA256 checksum of a file
    async fn verify_sha256(file_path: PathBuf, expected_hash: String) -> Result<PathBuf, String> {
        let contents = fs::read(&file_path)
            .await
            .map_err(|e| format!("Failed to read file for verification: {}", e))?;

        let mut hasher = Sha256::new();
        hasher.update(&contents);
        let result = hasher.finalize();
        let actual_hash = hex::encode(result);

        if actual_hash.to_lowercase() == expected_hash.to_lowercase() {
            Ok(file_path)
        } else {
            Err(format!(
                "SHA256 mismatch! Expected: {}, Got: {}",
                expected_hash, actual_hash
            ))
        }
    }

    /// Install the update based on the current platform
    async fn install(file_path: PathBuf) -> Result<(), String> {
        let os = Self::detect_os();

        match os {
            "macos" => Self::install_macos(file_path).await,
            "linux" => Self::install_deb(file_path).await,
            _ => Err(format!("Unsupported platform: {}", os)),
        }
    }

    /// Install the update on macOS
    async fn install_macos(file_path: PathBuf) -> Result<(), String> {
        let extension = file_path
            .extension()
            .and_then(|e| e.to_str())
            .ok_or_else(|| "Unknown file type".to_string())?;

        match extension {
            "dmg" => Self::install_dmg(file_path).await,
            "gz" if file_path.to_string_lossy().ends_with(".tar.gz") => {
                Self::install_tar_gz(file_path).await
            }
            "zip" => Self::install_zip(file_path).await,
            _ => Err(format!("Unsupported file type: {}", extension)),
        }
    }

    /// Install .deb package on Linux (Debian/Ubuntu)
    async fn install_deb(deb_path: PathBuf) -> Result<(), String> {
        let output = Command::new("pkexec")
            .args(["dpkg", "-i"])
            .arg(&deb_path)
            .output()
            .map_err(|e| format!("Failed to install .deb: {}", e))?;

        if output.status.success() {
            Ok(())
        } else {
            Err(format!(
                "Failed to install .deb package: {}",
                String::from_utf8_lossy(&output.stderr)
            ))
        }
    }

    /// Install from DMG file
    async fn install_dmg(dmg_path: PathBuf) -> Result<(), String> {
        // Mount the DMG
        let mount_output = Command::new("hdiutil")
            .args(["attach", "-nobrowse", "-readonly"])
            .arg(&dmg_path)
            .output()
            .map_err(|e| format!("Failed to mount DMG: {}", e))?;

        if !mount_output.status.success() {
            let stderr = String::from_utf8_lossy(&mount_output.stderr);
            return Err(format!("Failed to mount DMG: {}", stderr));
        }

        // Find mounted volume - parse hdiutil output
        // Format: /dev/diskX    TYPE    /Volumes/Name
        let mount_info = String::from_utf8_lossy(&mount_output.stdout);

        // Look for the last line that contains /Volumes/ - this is the actual mount point
        let volume_path = mount_info
            .lines()
            .rev() // Start from the end
            .find_map(|line| {
                // Split by tabs and whitespace, look for /Volumes/ entry
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    return None;
                }

                // The mount point is typically the last field after tabs
                let parts: Vec<&str> = line.split('\t').collect();
                for part in parts.iter().rev() {
                    let trimmed_part = part.trim();
                    if trimmed_part.starts_with("/Volumes/") {
                        return Some(trimmed_part.to_string());
                    }
                }
                None
            })
            .ok_or_else(|| {
                format!(
                    "Failed to find mount point in hdiutil output. Output was:\n{}",
                    mount_info
                )
            })?;

        // Verify the volume path exists
        if !PathBuf::from(&volume_path).exists() {
            return Err(format!(
                "Mount point '{}' does not exist. Full output:\n{}",
                volume_path, mount_info
            ));
        }

        // Find .app bundle in the mounted volume
        let app_bundle = Self::find_app_bundle(Path::new(&volume_path)).await?;

        // Copy to /Applications
        let app_name = app_bundle.file_name();
        let dest = PathBuf::from("/Applications").join(&app_name);

        // Remove existing app if present
        if dest.exists() {
            fs::remove_dir_all(&dest)
                .await
                .map_err(|e| format!("Failed to remove old app: {}", e))?;
        }

        // Copy new app
        let copy_output = Command::new("cp")
            .args(["-R"])
            .arg(app_bundle.path())
            .arg(&dest)
            .output()
            .map_err(|e| format!("Failed to copy app: {}", e))?;

        // Unmount DMG
        let detach_result = Command::new("hdiutil")
            .args(["detach", &volume_path])
            .output();

        // Check if copy was successful first
        if !copy_output.status.success() {
            let stderr = String::from_utf8_lossy(&copy_output.stderr);
            return Err(format!("Failed to copy app to Applications: {}", stderr));
        }

        // Log detach errors but don't fail the installation
        if let Ok(output) = detach_result {
            if !output.status.success() {
                eprintln!(
                    "Warning: Failed to detach DMG: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
            }
        }

        Ok(())
    }

    /// Install from tar.gz file
    async fn install_tar_gz(tar_gz_path: PathBuf) -> Result<(), String> {
        let extract_dir = tar_gz_path
            .parent()
            .ok_or_else(|| "Invalid tar.gz path".to_string())?;

        // Extract tar.gz
        let output = Command::new("tar")
            .args(["-xzf"])
            .arg(&tar_gz_path)
            .arg("-C")
            .arg(extract_dir)
            .output()
            .map_err(|e| format!("Failed to extract tar.gz: {}", e))?;

        if !output.status.success() {
            return Err("Failed to extract tar.gz".to_string());
        }

        // Find extracted .app bundle
        let app_bundle = Self::find_app_bundle(extract_dir).await?;

        // Copy to /Applications
        let app_name = app_bundle.file_name();
        let dest = PathBuf::from("/Applications").join(&app_name);

        // Remove existing app if present
        if dest.exists() {
            fs::remove_dir_all(&dest)
                .await
                .map_err(|e| format!("Failed to remove old app: {}", e))?;
        }

        // Copy new app
        let copy_output = Command::new("cp")
            .args(["-R"])
            .arg(app_bundle.path())
            .arg(&dest)
            .output()
            .map_err(|e| format!("Failed to copy app: {}", e))?;

        if copy_output.status.success() {
            Ok(())
        } else {
            Err("Failed to copy app to Applications".to_string())
        }
    }

    /// Install from zip file
    async fn install_zip(zip_path: PathBuf) -> Result<(), String> {
        let extract_dir = zip_path
            .parent()
            .ok_or_else(|| "Invalid zip path".to_string())?;

        // Extract zip
        let output = Command::new("unzip")
            .args(["-o", "-q"])
            .arg(&zip_path)
            .arg("-d")
            .arg(extract_dir)
            .output()
            .map_err(|e| format!("Failed to extract zip: {}", e))?;

        if !output.status.success() {
            return Err("Failed to extract zip".to_string());
        }

        // Find extracted .app bundle
        let app_bundle = Self::find_app_bundle(extract_dir).await?;

        // Copy to /Applications
        let app_name = app_bundle.file_name();
        let dest = PathBuf::from("/Applications").join(&app_name);

        // Remove existing app if present
        if dest.exists() {
            fs::remove_dir_all(&dest)
                .await
                .map_err(|e| format!("Failed to remove old app: {}", e))?;
        }

        // Copy new app
        let copy_output = Command::new("cp")
            .args(["-R"])
            .arg(app_bundle.path())
            .arg(&dest)
            .output()
            .map_err(|e| format!("Failed to copy app: {}", e))?;

        if copy_output.status.success() {
            Ok(())
        } else {
            Err("Failed to copy app to Applications".to_string())
        }
    }

    /// Find .app bundle in a directory
    async fn find_app_bundle(dir_path: &Path) -> Result<fs::DirEntry, String> {
        let mut read_dir = fs::read_dir(dir_path)
            .await
            .map_err(|e| format!("Failed to read directory '{}': {}", dir_path.display(), e))?;

        while let Some(entry) = read_dir.next_entry().await.transpose() {
            if let Ok(entry) = entry {
                if entry.path().extension().and_then(|e| e.to_str()) == Some("app") {
                    return Ok(entry);
                }
            }
        }

        Err(format!("No .app bundle found in '{}'", dir_path.display()))
    }

    /// Detect current OS
    fn detect_os() -> &'static str {
        #[cfg(target_os = "macos")]
        return "macos";

        #[cfg(target_os = "linux")]
        return "linux";

        #[cfg(target_os = "windows")]
        return "windows";

        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
        return "unknown";
    }

    /// Detect current architecture
    fn detect_arch() -> &'static str {
        #[cfg(target_arch = "x86_64")]
        return "x86_64";

        #[cfg(target_arch = "aarch64")]
        return "aarch64";

        #[cfg(target_arch = "x86")]
        return "x86";

        #[cfg(target_arch = "arm")]
        return "arm";

        #[cfg(not(any(
            target_arch = "x86_64",
            target_arch = "aarch64",
            target_arch = "x86",
            target_arch = "arm"
        )))]
        return "unknown";
    }

    /// Find the appropriate asset for the current platform
    fn find_platform_asset(&self, release: &ReleaseInfo) -> Option<ReleaseAsset> {
        let os = Self::detect_os();
        let arch = Self::detect_arch();

        // Common OS name variations
        let os_patterns = match os {
            "macos" => vec!["macos", "darwin", "osx", "mac"],
            "linux" => vec!["linux"],
            "windows" => vec!["windows", "win"],
            _ => vec![os],
        };

        // Common arch name variations
        let arch_patterns = match arch {
            "x86_64" => vec!["x86_64", "amd64", "x64"],
            "aarch64" => vec!["aarch64", "arm64"],
            "x86" => vec!["x86", "i686"],
            "arm" => vec!["arm", "armv7"],
            _ => vec![arch],
        };

        // Try to find an asset matching both OS and architecture
        release
            .assets
            .iter()
            .find(|asset| {
                let name = asset.name.to_lowercase();
                let has_os = os_patterns.iter().any(|pattern| name.contains(pattern));
                let has_arch = arch_patterns.iter().any(|pattern| name.contains(pattern));
                has_os && has_arch
            })
            .or_else(|| {
                // Fallback: try to find asset with just OS if no arch-specific one found
                release.assets.iter().find(|asset| {
                    let name = asset.name.to_lowercase();
                    os_patterns.iter().any(|pattern| name.contains(pattern))
                })
            })
            .cloned()
    }

    /// Find the SHA256 file for an asset (always {asset_name}.sha256)
    fn find_sha256_asset(&self, release: &ReleaseInfo, asset_name: &str) -> Option<ReleaseAsset> {
        let expected_name = format!("{}.sha256", asset_name);
        release
            .assets
            .iter()
            .find(|asset| asset.name == expected_name)
            .cloned()
    }
}

impl Plugin for AutoUpdaterPlugin {
    type Message = AutoUpdaterMessage;
    type State = AutoUpdaterState;
    type Output = AutoUpdaterOutput;

    fn name(&self) -> &'static str {
        "auto_updater"
    }

    fn init(&self) -> Self::State {
        AutoUpdaterState {
            download_progress: None,
            latest_release: None,
            is_updating: false,
            downloaded_file: None,
        }
    }

    fn update(
        &self,
        state: &mut Self::State,
        message: Self::Message,
    ) -> (Task<Self::Message>, Option<Self::Output>) {
        match message {
            AutoUpdaterMessage::CheckForUpdates => {
                let owner = self.config.owner.clone();
                let repo = self.config.repo.clone();
                let current_version = self.config.current_version.clone();

                let task = Task::perform(
                    Self::check_for_updates(owner, repo, current_version),
                    AutoUpdaterMessage::UpdateCheckResult,
                );

                (task, None)
            }

            AutoUpdaterMessage::UpdateCheckResult(result) => match result {
                Ok(Some(release)) => {
                    state.latest_release = Some(release.clone());
                    (
                        Task::none(),
                        Some(AutoUpdaterOutput::UpdateAvailable(release)),
                    )
                }
                Ok(None) => (Task::none(), Some(AutoUpdaterOutput::NoUpdateAvailable)),
                Err(e) => (Task::none(), Some(AutoUpdaterOutput::Error(e))),
            },

            AutoUpdaterMessage::DownloadAndInstall(release) => {
                if let Some(asset) = self.find_platform_asset(&release) {
                    state.is_updating = true;
                    state.latest_release = Some(release.clone());

                    let download_dir = self.download_dir();
                    let dest_path = download_dir.join(&asset.name);
                    let url = asset.browser_download_url.clone();

                    let task = Task::perform(
                        Self::download_file(url, dest_path),
                        AutoUpdaterMessage::DownloadCompleted,
                    );

                    (task, Some(AutoUpdaterOutput::DownloadStarted(release)))
                } else {
                    (
                        Task::none(),
                        Some(AutoUpdaterOutput::Error(format!(
                            "No suitable asset found for {} {}",
                            Self::detect_os(),
                            Self::detect_arch()
                        ))),
                    )
                }
            }

            AutoUpdaterMessage::DownloadCompleted(result) => match result {
                Ok(path) => {
                    state.downloaded_file = Some(path.clone());

                    // Find SHA256 asset
                    if let Some(release) = &state.latest_release {
                        if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                            if let Some(sha256_asset) = self.find_sha256_asset(release, file_name) {
                                // Download and verify SHA256
                                let file_path = path.clone();
                                let sha256_url = sha256_asset.browser_download_url.clone();

                                let task = Task::perform(
                                    async move {
                                        let expected_hash =
                                            Self::download_sha256(sha256_url).await?;
                                        Self::verify_sha256(file_path, expected_hash).await
                                    },
                                    AutoUpdaterMessage::VerificationResult,
                                );

                                return (task, None);
                            }
                        }
                    }

                    // If no SHA256 file found, skip verification and install
                    let task = Task::perform(
                        Self::install(path.clone()),
                        AutoUpdaterMessage::InstallationResult,
                    );

                    (task, Some(AutoUpdaterOutput::DownloadCompleted(path)))
                }
                Err(e) => {
                    state.is_updating = false;
                    (Task::none(), Some(AutoUpdaterOutput::Error(e)))
                }
            },

            AutoUpdaterMessage::VerificationResult(result) => match result {
                Ok(path) => {
                    let task = Task::perform(
                        Self::install(path.clone()),
                        AutoUpdaterMessage::InstallationResult,
                    );

                    (task, Some(AutoUpdaterOutput::VerificationSucceeded(path)))
                }
                Err(e) => {
                    state.is_updating = false;
                    state.downloaded_file = None;
                    (Task::none(), Some(AutoUpdaterOutput::VerificationFailed(e)))
                }
            },

            AutoUpdaterMessage::InstallationResult(result) => {
                state.is_updating = false;
                state.downloaded_file = None;

                match result {
                    Ok(()) => (Task::none(), Some(AutoUpdaterOutput::InstallationCompleted)),
                    Err(e) => (Task::none(), Some(AutoUpdaterOutput::Error(e))),
                }
            }

            AutoUpdaterMessage::AutoCheckTick => {
                // Only check if not currently updating
                if !state.is_updating {
                    let owner = self.config.owner.clone();
                    let repo = self.config.repo.clone();
                    let current_version = self.config.current_version.clone();

                    let task = Task::perform(
                        Self::check_for_updates(owner, repo, current_version),
                        AutoUpdaterMessage::UpdateCheckResult,
                    );

                    (task, None)
                } else {
                    (Task::none(), None)
                }
            }

            AutoUpdaterMessage::DownloadProgress(progress) => {
                state.download_progress = Some(progress.clone());
                (
                    Task::none(),
                    Some(AutoUpdaterOutput::DownloadProgress(progress)),
                )
            }
        }
    }

    fn subscription(&self, _state: &Self::State) -> Subscription<Self::Message> {
        if self.config.auto_check_interval > 0 {
            every(Duration::from_secs(self.config.auto_check_interval))
                .map(|_| AutoUpdaterMessage::AutoCheckTick)
        } else {
            Subscription::none()
        }
    }
}
