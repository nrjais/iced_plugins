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
//! - **Required** SHA256 checksum verification for security
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

mod macos;

use iced::task::{Straw, sipper};
use iced::time::every;
use iced::{Subscription, Task};
use iced_plugins::Plugin;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use std::time::Duration;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

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
    /// Check for updates on application start
    pub check_on_start: bool,
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
            auto_check_interval: 0,
            check_on_start: false,
        }
    }

    /// Enable automatic update checking with specified interval
    pub fn with_auto_check(mut self, interval_secs: u64) -> Self {
        self.auto_check_interval = interval_secs;
        self
    }

    /// Enable checking for updates on application start
    pub fn with_check_on_start(mut self, enabled: bool) -> Self {
        self.check_on_start = enabled;
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
    pub total_size: u64,
}

impl DownloadProgress {
    pub fn percentage(&self) -> f32 {
        if self.total_size == 0 {
            0.0
        } else {
            (self.downloaded as f32 / self.total_size as f32) * 100.0
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
    /// Start installation
    StartInstallation(PathBuf),
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
    /// Download Failed
    DownloadFailed(String),
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
#[derive(Debug, Clone)]
pub struct AutoUpdaterState {
    /// Current download progress
    pub download_progress: Option<DownloadProgress>,
    /// Latest release info
    pub latest_release: Option<ReleaseInfo>,
    /// Whether an update is being processed
    pub is_updating: bool,
    /// Downloaded file path
    pub downloaded_file: Option<PathBuf>,
    /// Abort handle of the download task
    pub abort_handle: Option<iced::task::Handle>,
}

/// Auto updater plugin
#[derive(Debug, Clone)]
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

        let latest_version = release.tag_name.trim_start_matches('v');
        let current = current_version.trim_start_matches('v');

        if latest_version != current {
            Ok(Some(release))
        } else {
            Ok(None)
        }
    }

    fn download_file(
        url: String,
        dest_path: PathBuf,
    ) -> impl Straw<PathBuf, DownloadProgress, String> {
        sipper(move |mut progress| async move {
            use futures_util::stream::StreamExt;

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

            let total_size = response
                .content_length()
                .ok_or_else(|| "Failed to get content length".to_string())?;

            if let Some(parent) = dest_path.parent() {
                fs::create_dir_all(parent)
                    .await
                    .map_err(|e| format!("Failed to create download directory: {}", e))?;
            }

            let mut file = fs::File::create(&dest_path)
                .await
                .map_err(|e| format!("Failed to create file: {}", e))?;

            let mut stream = response.bytes_stream();
            let mut downloaded: u64 = 0;

            while let Some(chunk_result) = stream.next().await {
                let chunk = chunk_result.map_err(|e| format!("Failed to read chunk: {}", e))?;

                file.write_all(&chunk)
                    .await
                    .map_err(|e| format!("Failed to write chunk: {}", e))?;

                downloaded += chunk.len() as u64;
                let _ = progress
                    .send(DownloadProgress {
                        downloaded,
                        total_size,
                    })
                    .await;
            }

            file.flush()
                .await
                .map_err(|e| format!("Failed to flush file: {}", e))?;

            Ok(dest_path)
        })
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
            "macos" => macos::install(file_path).await,
            "linux" => Self::install_deb(file_path).await,
            _ => Err(format!("Unsupported platform: {}", os)),
        }
    }

    /// Install .deb package on Linux (Debian/Ubuntu)
    async fn install_deb(deb_path: PathBuf) -> Result<(), String> {
        let output = Command::new("pkexec")
            .args(["dpkg", "-i"])
            .arg(&deb_path)
            .output()
            .await
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

        let os_patterns = match os {
            "macos" => vec!["macos", "darwin", "osx", "mac"],
            "linux" => vec!["linux"],
            "windows" => vec!["windows", "win"],
            _ => vec![os],
        };

        let arch_patterns = match arch {
            "x86_64" => vec!["x86_64", "amd64", "x64"],
            "aarch64" => vec!["aarch64", "arm64"],
            "x86" => vec!["x86", "i686"],
            "arm" => vec!["arm", "armv7"],
            _ => vec![arch],
        };

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

    fn init(&self) -> (Self::State, Task<Self::Message>) {
        let state = AutoUpdaterState {
            download_progress: None,
            latest_release: None,
            is_updating: false,
            downloaded_file: None,
            abort_handle: None,
        };

        let init_task = if self.config.check_on_start {
            let owner = self.config.owner.clone();
            let repo = self.config.repo.clone();
            let current_version = self.config.current_version.clone();

            Task::perform(
                Self::check_for_updates(owner, repo, current_version),
                AutoUpdaterMessage::UpdateCheckResult,
            )
        } else {
            Task::none()
        };

        (state, init_task)
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

                    let (task, handle) = Task::sip(
                        Self::download_file(url, dest_path),
                        AutoUpdaterMessage::DownloadProgress,
                        AutoUpdaterMessage::DownloadCompleted,
                    )
                    .abortable();
                    state.abort_handle = Some(handle.abort_on_drop());

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

                    let output = Some(AutoUpdaterOutput::DownloadCompleted(path.clone()));

                    if let Some(release) = &state.latest_release
                        && let Some(file_name) = path.file_name().and_then(|n| n.to_str())
                    {
                        if let Some(sha256_asset) = self.find_sha256_asset(release, file_name) {
                            let file_path = path.clone();
                            let sha256_url = sha256_asset.browser_download_url.clone();

                            let task = Task::perform(
                                async move {
                                    let expected_hash = Self::download_sha256(sha256_url).await?;
                                    Self::verify_sha256(file_path, expected_hash).await
                                },
                                AutoUpdaterMessage::VerificationResult,
                            );

                            return (task, output);
                        } else {
                            state.is_updating = false;
                            state.downloaded_file = None;
                            return (
                                Task::none(),
                                Some(AutoUpdaterOutput::DownloadFailed(format!(
                                    "SHA256 checksum file not found for {}. Verification is required.",
                                    file_name
                                ))),
                            );
                        }
                    }

                    state.is_updating = false;
                    state.downloaded_file = None;
                    (
                        Task::none(),
                        Some(AutoUpdaterOutput::Error(
                            "Unable to verify download: missing release information".to_string(),
                        )),
                    )
                }
                Err(e) => {
                    state.is_updating = false;
                    (Task::none(), Some(AutoUpdaterOutput::Error(e)))
                }
            },

            AutoUpdaterMessage::VerificationResult(result) => match result {
                Ok(path) => (
                    Task::done(AutoUpdaterMessage::StartInstallation(path.clone())),
                    Some(AutoUpdaterOutput::VerificationSucceeded(path)),
                ),
                Err(e) => {
                    state.is_updating = false;
                    state.downloaded_file = None;
                    (Task::none(), Some(AutoUpdaterOutput::VerificationFailed(e)))
                }
            },

            AutoUpdaterMessage::StartInstallation(path) => {
                let task =
                    Task::perform(Self::install(path), AutoUpdaterMessage::InstallationResult);

                (task, Some(AutoUpdaterOutput::InstallationStarted))
            }

            AutoUpdaterMessage::InstallationResult(result) => {
                state.is_updating = false;
                state.downloaded_file = None;

                match result {
                    Ok(()) => (Task::none(), Some(AutoUpdaterOutput::InstallationCompleted)),
                    Err(e) => (Task::none(), Some(AutoUpdaterOutput::Error(e))),
                }
            }

            AutoUpdaterMessage::AutoCheckTick => {
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
