# Auto Updater Plugin for Iced

An automatic update plugin for Iced applications that checks for updates from GitHub releases, downloads them, verifies SHA256 checksums, and installs them.

## Features

- ✅ Check for updates from GitHub releases
- ✅ Automatic OS and architecture detection
- ✅ Download release assets
- ✅ Verify SHA256 checksums
- ✅ Install bundles on multiple platforms
- ✅ Progress tracking for downloads
- ✅ Automatic or manual update checks
- ✅ Type-safe plugin API

## Platform Support

- **macOS**: Full support for .dmg, .tar.gz, and .zip bundles
- **Linux**: Support for .deb packages (Debian/Ubuntu)

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
iced_auto_updater_plugin = { path = "path/to/iced_plugins/plugins/auto_updater" }
iced_plugins = { path = "path/to/iced_plugins" }
```

## Quick Start

```rust
use iced_auto_updater_plugin::{AutoUpdaterPlugin, AutoUpdaterMessage, AutoUpdaterOutput, UpdaterConfig};
use iced_plugins::{PluginManager, PluginHandle};

const APP_NAME: &str = "my_app";
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

struct App {
    plugins: PluginManager,
    updater_handle: PluginHandle<AutoUpdaterPlugin>,
}

impl App {
    fn new() -> (Self, Task<Message>) {
        // Configure the auto updater
        let config = UpdaterConfig::new(
            "your-github-username",
            "your-repo-name",
            CURRENT_VERSION
        )
        .with_auto_check(3600)      // Check every hour
        .with_check_on_start(true); // Check on app start

        // Use the builder pattern to set up plugins
        let (plugins, init_task) = PluginManagerBuilder::new()
            .with_plugin(AutoUpdaterPlugin::new(APP_NAME.to_string(), config))
            .build();

        // Retrieve handle after building
        let updater_handle = plugins.get_handle::<AutoUpdaterPlugin>().unwrap();

        (Self { plugins, updater_handle }, init_task.map(Message::Plugin))
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Plugin(plugin_msg) => {
                self.plugins.update(plugin_msg).map(Message::Plugin)
            }
            Message::CheckForUpdates => {
                self.updater_handle
                    .dispatch(AutoUpdaterMessage::CheckForUpdates)
                    .map(Message::Plugin)
            }
            Message::UpdaterOutput(output) => {
                match output {
                    AutoUpdaterOutput::UpdateAvailable(release) => {
                        // Notify user of available update
                        println!("Update available: {}", release.name);
                    }
                    AutoUpdaterOutput::DownloadProgress(progress) => {
                        println!("Download: {:.1}%", progress.percentage());
                    }
                    AutoUpdaterOutput::InstallationCompleted => {
                        println!("Update installed! Please restart.");
                    }
                    _ => {}
                }
                Task::none()
            }
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            self.plugins.subscriptions().map(Message::Plugin),
            self.updater_handle.listen().map(Message::UpdaterOutput),
        ])
    }
}
```

## Configuration

### UpdaterConfig

Configure the auto updater with `UpdaterConfig`:

```rust
let config = UpdaterConfig::new(owner, repo, current_version)
    .with_auto_check(3600)      // Auto-check interval in seconds (0 = disabled)
    .with_check_on_start(true); // Check for updates on application start
```

**Configuration options:**
- `with_auto_check(interval_secs)` - Enable periodic update checks. Set to 0 to disable. For example, `3600` checks every hour.
- `with_check_on_start(enabled)` - Check for updates when the application starts. Default is `false`.

**Using check_on_start:**

```rust
impl App {
    fn new() -> (Self, Task<Message>) {
        let config = UpdaterConfig::new("owner", "repo", CURRENT_VERSION)
            .with_check_on_start(true);  // Enable check on start

        // The plugin's init method will automatically check for updates
        // when check_on_start is enabled
        let (plugins, init_task) = PluginManagerBuilder::new()
            .with_plugin(AutoUpdaterPlugin::new(APP_NAME.to_string(), config))
            .build();

        // Retrieve handle after building
        let updater_handle = plugins.get_handle::<AutoUpdaterPlugin>().unwrap();

        // Map the init task to your app's message type
        (Self { plugins, updater_handle }, init_task.map(Message::Plugin))
    }
}
```

### Asset Naming

The plugin **automatically detects** the current OS and architecture to find the appropriate asset in GitHub releases.

**Supported OS naming conventions:**
- **macOS**: `macos`, `darwin`, `osx`, `mac`
- **Linux**: `linux` (Debian/Ubuntu with .deb packages)

**Supported architecture naming conventions:**
- **x86_64**: `x86_64`, `amd64`, `x64`
- **aarch64**: `aarch64`, `arm64`
- **x86**: `x86`, `i686`
- **arm**: `arm`, `armv7`

**Example release structure:**
```
Release v1.0.0:
  - myapp-macos-aarch64-v1.0.0.tar.gz        (Apple Silicon)
  - myapp-macos-aarch64-v1.0.0.tar.gz.sha256 (checksum)
  - myapp-macos-x86_64-v1.0.0.tar.gz         (Intel Mac)
  - myapp-macos-x86_64-v1.0.0.tar.gz.sha256  (checksum)
  - myapp-linux-x86_64-v1.0.0.deb            (Linux Debian/Ubuntu)
  - myapp-linux-x86_64-v1.0.0.deb.sha256     (checksum)
```

The plugin will automatically select the correct asset based on the platform it's running on.

## Messages

### AutoUpdaterMessage

Messages you can send to the plugin:

- `CheckForUpdates` - Manually check for updates
- `DownloadAndInstall(ReleaseInfo)` - Download and install a specific release

```rust
// Check for updates
let task = updater_handle.dispatch(AutoUpdaterMessage::CheckForUpdates);

// Download and install when update is available
let task = updater_handle.dispatch(
    AutoUpdaterMessage::DownloadAndInstall(release)
);
```

### AutoUpdaterOutput

Output events you can listen to:

- `UpdateAvailable(ReleaseInfo)` - A new version is available
- `NoUpdateAvailable` - Already on latest version
- `DownloadStarted(ReleaseInfo)` - Download has begun
- `DownloadProgress(DownloadProgress)` - Download progress update
- `DownloadCompleted(PathBuf)` - Download finished
- `VerificationSucceeded(PathBuf)` - Verification passed
- `VerificationFailed(String)` - Verification failed
- `InstallationStarted` - Starting installation
- `InstallationCompleted` - Installation finished
- `Error(String)` - An error occurred

## SHA256 Verification

The plugin automatically verifies downloads using SHA256 checksums:

1. Downloads the main asset (e.g., `myapp-macos-aarch64.tar.gz`)
2. Looks for the SHA256 file with `.sha256` extension (e.g., `myapp-macos-aarch64.tar.gz.sha256`)
3. Downloads the SHA256 file
4. Verifies the downloaded bundle matches the expected hash
5. Only proceeds with installation if verification succeeds

**SHA256 file naming:** The SHA256 file must be named exactly as the asset file plus `.sha256` extension.

**SHA256 file format:**
```
a1b2c3d4e5f6...  myapp-macos-aarch64-v1.0.0.tar.gz
```
Or just the hash:
```
a1b2c3d4e5f6...
```

## Platform Installation

### macOS

The plugin supports three bundle formats on macOS:

**DMG Files (.dmg)**
1. Mounts the DMG
2. Finds the .app bundle inside
3. Copies to /Applications
4. Unmounts the DMG

**Tar Gzip Archives (.tar.gz)**
1. Extracts the archive
2. Finds the .app bundle
3. Copies to /Applications

**Zip Archives (.zip)**
1. Extracts the archive
2. Finds the .app bundle
3. Copies to /Applications

The plugin automatically removes existing versions before installing updates.

### Linux (Debian/Ubuntu)

**DEB Packages (.deb)**
1. Uses `pkexec` for elevated privileges
2. Runs `dpkg -i` to install the package
3. Handles system package installation

Note: The plugin requires `pkexec` (part of PolicyKit) for privilege escalation.

## GitHub Release Setup

To use this plugin, your GitHub releases should include:

1. **Tagged releases** (e.g., `v1.0.0`, `1.0.0`)
2. **Platform-specific bundles** as release assets (with OS and architecture in the filename)
3. **SHA256 checksum files** for each bundle (with `.sha256` extension)


## Example

See [`examples/auto_updater_plugin.rs`](../../examples/auto_updater_plugin.rs) for a complete example application.

Run it with:
```sh
cargo run --example auto_updater_plugin
```
