# Auto Updater Plugin for Iced

An automatic update plugin for Iced applications that checks for updates from GitHub releases, downloads them, verifies SHA256 checksums, and installs them.

## Features

- Check for updates from GitHub releases with automatic OS and architecture detection
- Download and verify release assets with **required** SHA256 checksum verification
- Install updates on macOS (.dmg, .tar.gz, .zip) and Linux (.deb)
- Progress tracking for downloads
- Configurable automatic or manual update checks

## Platform Support

- **macOS**: Full support for .dmg, .tar.gz, and .zip bundles
- **Linux**: Support for .deb packages (Debian/Ubuntu)

## Usage

See [`examples/auto_updater_plugin.rs`](../../examples/auto_updater_plugin.rs) for a complete example.

Run the example with:
```sh
cargo run --example auto_updater_plugin
```

## Quick Start

```rust
use iced_auto_updater_plugin::{AutoUpdaterPlugin, UpdaterConfig};

let config = UpdaterConfig::new("owner", "repo", "1.0.0")
    .with_auto_check(3600)      // Check every hour
    .with_check_on_start(true); // Check on app start

let (plugins, init_task) = PluginManagerBuilder::new()
    .with_plugin(AutoUpdaterPlugin::new("my_app".to_string(), config))
    .build();
```

## GitHub Release Setup

Your GitHub releases should include:
1. Tagged releases (e.g., `v1.0.0`)
2. Platform-specific bundles with OS and architecture in the filename (e.g., `myapp-macos-aarch64-v1.0.0.tar.gz`)
3. SHA256 checksum files for each bundle with `.sha256` extension

### Creating SHA256 Checksums

Generate checksum files for your release assets:

```bash
# macOS/Linux
shasum -a 256 myapp-macos-aarch64.dmg > myapp-macos-aarch64.dmg.sha256

# Or use sha256sum on Linux
sha256sum myapp-linux-x86_64.deb > myapp-linux-x86_64.deb.sha256
```

The `.sha256` file should contain only the hash and filename on a single line.
