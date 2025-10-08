//! Example application demonstrating the Auto Updater Plugin
//!
//! This example shows a complete end-to-end update flow:
//! 1. Configure the auto updater plugin with automatic OS/arch detection
//! 2. Check for updates on startup (with check_on_start enabled)
//! 3. Check for updates from GitHub releases
//! 4. Display available update information
//! 5. Download the platform-specific asset
//! 6. Verify SHA256 checksum
//! 7. Install the update
//!
//! Features demonstrated:
//! - Automatic update checking on application start
//! - Periodic update checks (every hour)
//! - Manual update checks
//! - Platform detection (OS and architecture)
//! - Download progress tracking
//! - SHA256 verification
//! - Installation flow
//!
//! To run this example:
//! ```sh
//! cargo run --example auto_updater_plugin
//! ```

use iced::widget::{Column, Row, button, progress_bar, scrollable, text};
use iced::{Element, Fill, Length, Subscription, Task};
use iced_auto_updater_plugin::{
    AutoUpdaterMessage, AutoUpdaterOutput, AutoUpdaterPlugin, ReleaseInfo, UpdaterConfig,
};
use iced_plugins::{PluginHandle, PluginManager, PluginManagerBuilder, PluginMessage};

const APP_NAME: &str = "auto_updater_example";
const CURRENT_VERSION: &str = "0.1.0";

// Update flow steps
#[derive(Debug, Clone, PartialEq)]
enum UpdateStep {
    Idle,
    Checking,
    UpdateAvailable,
    Downloading,
    Downloaded,
    Verifying,
    Verified,
    Installing,
    Completed,
    Failed,
}

fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .subscription(App::subscription)
        .run()
}

struct App {
    plugins: PluginManager,
    updater_handle: PluginHandle<AutoUpdaterPlugin>,
    current_step: UpdateStep,
    status_message: String,
    available_update: Option<ReleaseInfo>,
    download_progress: f32,
    event_log: Vec<String>,
    detected_platform: String,
}

#[derive(Debug, Clone)]
enum Message {
    Plugin(PluginMessage),
    UpdaterOutput(AutoUpdaterOutput),
    CheckForUpdates,
    DownloadAndInstall,
}

impl App {
    fn new() -> (Self, Task<Message>) {
        // Detect current platform
        let os = detect_os();
        let arch = detect_arch();
        let platform_info = format!("{} ({})", os, arch);

        // Configure the auto updater
        // Replace with your actual GitHub repo
        let config = UpdaterConfig::new("nrjais", "sanchaar", CURRENT_VERSION)
            // Enable auto-check every hour (3600 seconds)
            .with_auto_check(3600)
            // Enable check on start - this will automatically check for updates when the app starts
            .with_check_on_start(true);

        let check_on_start = config.check_on_start;

        // Use the builder pattern to set up plugins
        let (plugins, init_task) = PluginManagerBuilder::new()
            .with_plugin(AutoUpdaterPlugin::new(APP_NAME.to_string(), config))
            .build();

        // Retrieve handle after building
        let updater_handle = plugins.get_handle::<AutoUpdaterPlugin>().unwrap();

        let mut event_log = Vec::new();
        event_log.push(format!(
            "🚀 Auto Updater initialized for {} {}",
            APP_NAME, CURRENT_VERSION
        ));
        event_log.push(format!("💻 Platform detected: {}", platform_info));
        event_log.push(format!(
            "📦 Will look for assets matching: {}",
            platform_info.to_lowercase()
        ));

        if check_on_start {
            event_log.push("✨ Checking for updates on startup...".to_string());
        } else {
            event_log.push("✨ Ready to check for updates".to_string());
        }

        (
            Self {
                plugins,
                updater_handle,
                current_step: if check_on_start {
                    UpdateStep::Checking
                } else {
                    UpdateStep::Idle
                },
                status_message: if check_on_start {
                    "Checking for updates...".to_string()
                } else {
                    "Ready to check for updates".to_string()
                },
                available_update: None,
                download_progress: 0.0,
                event_log,
                detected_platform: platform_info,
            },
            init_task.map(Message::Plugin),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Plugin(plugin_msg) => self.plugins.update(plugin_msg).map(Message::Plugin),

            Message::UpdaterOutput(output) => {
                match output {
                    AutoUpdaterOutput::UpdateAvailable(release) => {
                        self.current_step = UpdateStep::UpdateAvailable;
                        self.status_message = format!(
                            "✅ Update available: {} ({})",
                            release.name, release.tag_name
                        );

                        self.available_update = Some(release);
                    }
                    AutoUpdaterOutput::NoUpdateAvailable => {
                        self.current_step = UpdateStep::Idle;
                        self.status_message =
                            format!("✅ You're on the latest version ({})", CURRENT_VERSION);

                        self.available_update = None;
                    }
                    AutoUpdaterOutput::DownloadStarted(_release) => {
                        self.current_step = UpdateStep::Downloading;
                        self.status_message = "⬇️ Downloading update...".to_string();
                        self.download_progress = 0.0;
                    }
                    AutoUpdaterOutput::DownloadProgress(progress) => {
                        self.download_progress = progress.percentage();
                        self.status_message = format!(
                            "⬇️ Downloading: {:.1}% ({} / {} bytes)",
                            progress.percentage(),
                            progress.downloaded,
                            progress.total_size
                        );
                    }
                    AutoUpdaterOutput::DownloadCompleted(path) => {
                        self.current_step = UpdateStep::Downloaded;
                        self.status_message =
                            format!("✅ Download completed ({})", path.display()).to_string();
                        self.download_progress = 100.0;
                    }
                    AutoUpdaterOutput::VerificationSucceeded(path) => {
                        self.current_step = UpdateStep::Verified;
                        self.status_message =
                            format!("✅ Verification successful ({})", path.display()).to_string();
                    }
                    AutoUpdaterOutput::VerificationFailed(err) => {
                        self.current_step = UpdateStep::Failed;
                        self.status_message = format!("❌ Verification failed: {}", err);
                    }
                    AutoUpdaterOutput::InstallationStarted => {
                        self.current_step = UpdateStep::Installing;
                        self.status_message = "📦 Installing update...".to_string();
                    }
                    AutoUpdaterOutput::InstallationCompleted => {
                        self.current_step = UpdateStep::Completed;
                        self.status_message = "🎉 Update installed successfully!".to_string();

                        self.available_update = None;
                    }
                    AutoUpdaterOutput::Error(err) => {
                        println!("❌ Error: {}", err);
                        self.current_step = UpdateStep::Failed;
                        self.status_message = format!("❌ Error: {}", err);
                    }
                    _ => {}
                }
                Task::none()
            }

            Message::CheckForUpdates => {
                self.current_step = UpdateStep::Checking;
                self.status_message = "🔍 Checking for updates...".to_string();

                self.updater_handle
                    .dispatch(AutoUpdaterMessage::CheckForUpdates)
                    .map(Message::Plugin)
            }

            Message::DownloadAndInstall => {
                if let Some(release) = self.available_update.clone() {
                    self.updater_handle
                        .dispatch(AutoUpdaterMessage::DownloadAndInstall(release))
                        .map(Message::Plugin)
                } else {
                    Task::none()
                }
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        // Header
        let title = text("🔄 Auto Updater - End-to-End Flow Example")
            .size(28)
            .width(Fill);

        let version_row = Row::new()
            .push(text("Current Version:").size(14))
            .push(text(CURRENT_VERSION).size(14))
            .push(text("│").size(14))
            .push(text("Platform:").size(14))
            .push(text(&self.detected_platform).size(14))
            .spacing(8);

        // Status section with step indicator
        let step_indicator = self.get_step_indicator();

        let status_section = Column::new()
            .push(text("Update Status").size(18))
            .push(text("━".repeat(60)).size(10))
            .push(step_indicator)
            .push(text(&self.status_message).size(16))
            .spacing(8)
            .padding(10);

        // Progress bar for downloads
        let progress_section = if matches!(self.current_step, UpdateStep::Downloading) {
            Column::new()
                .push(progress_bar(0.0..=100.0, self.download_progress))
                .push(text(format!("{:.1}%", self.download_progress)).size(12))
                .spacing(4)
        } else {
            Column::new()
        };

        // Action buttons
        let buttons = self.get_action_buttons();

        // Release info section
        let release_section = if let Some(release) = &self.available_update {
            let mut col = Column::new()
                .push(text("Available Update").size(18))
                .push(text("━".repeat(60)).size(10))
                .push(text(format!("Version: {}", release.tag_name)).size(14))
                .push(text(format!("Name: {}", release.name)).size(14))
                .spacing(4);

            if let Some(body) = &release.body {
                let notes = body.lines().take(5).collect::<Vec<_>>().join("\n");
                col = col
                    .push(text("Release Notes:").size(14))
                    .push(text(notes).size(12));
            }

            col.padding(10)
        } else {
            Column::new()
        };

        // Event log
        let event_log = self.get_event_log();

        // Event log section
        let log_section = Column::new()
            .push(text("Event Log").size(18))
            .push(text("━".repeat(60)).size(10))
            .push(event_log)
            .spacing(8)
            .padding(10);

        // Main layout
        let content = Column::new()
            .push(title)
            .push(version_row)
            .push(text("━".repeat(80)).size(10))
            .push(status_section)
            .push(progress_section)
            .push(buttons)
            .push(text("━".repeat(80)).size(10))
            .push(release_section)
            .push(text("━".repeat(80)).size(10))
            .push(log_section)
            .spacing(15)
            .padding(20)
            .width(Fill);

        scrollable(content).width(Fill).height(Fill).into()
    }

    fn get_step_indicator(&self) -> Element<'_, Message> {
        let steps = vec![
            ("Idle", UpdateStep::Idle),
            ("Checking", UpdateStep::Checking),
            ("Update Available", UpdateStep::UpdateAvailable),
            ("Downloading", UpdateStep::Downloading),
            ("Downloaded", UpdateStep::Downloaded),
            ("Verifying", UpdateStep::Verifying),
            ("Verified", UpdateStep::Verified),
            ("Installing", UpdateStep::Installing),
            ("Completed", UpdateStep::Completed),
        ];

        let mut row = Row::new().spacing(4);

        for (label, step) in steps {
            let symbol = if self.current_step == step {
                "●" // Current step
            } else if self.is_step_completed(&step) {
                "✓" // Completed step
            } else if step == UpdateStep::Failed && self.current_step == UpdateStep::Failed {
                "✗" // Failed
            } else {
                "○" // Not started
            };

            row = row.push(text(format!("{} {}", symbol, label)).size(11));
        }

        row.into()
    }

    fn is_step_completed(&self, step: &UpdateStep) -> bool {
        use UpdateStep::*;
        match (&self.current_step, step) {
            (Checking, Idle) => true,
            (UpdateAvailable, Idle | Checking) => true,
            (Downloading, Idle | Checking | UpdateAvailable) => true,
            (Downloaded, Idle | Checking | UpdateAvailable | Downloading) => true,
            (Verifying, Idle | Checking | UpdateAvailable | Downloading | Downloaded) => true,
            (
                Verified,
                Idle | Checking | UpdateAvailable | Downloading | Downloaded | Verifying,
            ) => true,
            (
                Installing,
                Idle | Checking | UpdateAvailable | Downloading | Downloaded | Verifying | Verified,
            ) => true,
            (Completed, _) => step != &Completed,
            _ => false,
        }
    }

    fn get_action_buttons(&self) -> Element<'_, Message> {
        let mut buttons = Row::new().spacing(10).padding(10);

        match self.current_step {
            UpdateStep::Idle | UpdateStep::Failed | UpdateStep::Completed => {
                buttons = buttons.push(
                    button(text("🔍 Check for Updates").size(14))
                        .on_press(Message::CheckForUpdates),
                );
            }
            UpdateStep::UpdateAvailable => {
                buttons = buttons.push(
                    button(text("⬇️ Download and Install").size(14))
                        .on_press(Message::DownloadAndInstall),
                );
                buttons = buttons.push(
                    button(text("🔍 Check Again").size(14)).on_press(Message::CheckForUpdates),
                );
            }
            _ => {
                buttons = buttons.push(button(text("⏳ Processing...").size(14)));
            }
        }

        buttons.into()
    }

    fn get_event_log(&self) -> Element<'_, Message> {
        let mut log_col = Column::new().spacing(2);

        // Show last 15 events
        for event in self.event_log.iter().rev().take(15).rev() {
            log_col = log_col.push(text(event).size(11));
        }

        scrollable(log_col).height(Length::Fixed(200.0)).into()
    }

    fn subscription(&self) -> Subscription<Message> {
        let updater_sub = self
            .plugins
            .get_handle::<AutoUpdaterPlugin>()
            .map(|handle| handle.listen().map(Message::UpdaterOutput));
        Subscription::batch([
            self.plugins.subscriptions().map(Message::Plugin),
            updater_sub.unwrap_or(Subscription::none()),
        ])
    }
}

// Helper functions to detect platform (same logic as the plugin)
fn detect_os() -> &'static str {
    #[cfg(target_os = "macos")]
    return "macOS";

    #[cfg(target_os = "linux")]
    return "Linux";

    #[cfg(target_os = "windows")]
    return "Windows";

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    return "Unknown";
}

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
