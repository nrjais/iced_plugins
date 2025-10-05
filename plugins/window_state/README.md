# Iced Window State Plugin

A plugin for [Iced](https://github.com/iced-rs/iced) applications that automatically saves and restores window state (size, position, maximized status) to disk.

## Features

- 🪟 **Automatic State Persistence**: Window size and position are automatically saved
- 💾 **Disk Storage**: State is saved to a JSON file in the user's config directory
- 🔄 **Load Before Creation**: Window state can be loaded before app initialization
- ⏱️ **Debounced Saves**: Auto-save with configurable interval to avoid excessive I/O
- 🎯 **Multi-Window Support**: Support for tracking multiple windows
- 🛡️ **Type-Safe**: Full type safety with the iced_plugins system

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
iced_window_state_plugin = { path = "plugins/window_state" }
iced_plugins = { path = "." }
```

## Quick Start

### 1. Load State Before App Creation

```rust
use iced::{Settings, window};
use iced_window_state_plugin::WindowStatePlugin;

fn main() -> iced::Result {
    // Load saved window state
    let window_state = WindowStatePlugin::load_from_disk();

    // Use it to configure the window
    let settings = Settings {
        window: window::Settings {
            size: window_state.size,
            position: window_state.position,
            ..Default::default()
        },
        ..Default::default()
    };

    App::run(settings)
}
```

### 2. Install the Plugin

```rust
use iced_plugins::PluginManager;
use iced_window_state_plugin::{WindowStatePlugin, WindowStateMessage};

#[derive(Debug, Clone)]
enum Message {
    WindowState(WindowStateMessage),
    // ... other messages
}

struct App {
    plugins: PluginManager<Message>,
}

impl Application for App {
    fn new(_flags: ()) -> (Self, Command<Message>) {
        let mut plugins = PluginManager::new();

        // Install the window state plugin
        plugins.install(WindowStatePlugin::new(), Message::WindowState);

        (App { plugins }, Command::none())
    }

    fn subscription(&self) -> Subscription<Message> {
        // This will include window resize events and auto-save timer
        self.plugins.subscriptions()
    }

    // ... rest of implementation
}
```

### 3. Handle Plugin Messages

```rust
fn update(&mut self, message: Message) -> Command<Message> {
    match message {
        Message::WindowState(msg) => {
            if let Some(state) = self.plugins
                .get_typed_state_mut::<WindowPluginState>("window_state")
            {
                let plugin = WindowStatePlugin::new();
                let _ = plugin.update(state, msg);
            }
        }
        // ... other messages
    }
    Command::none()
}
```

## Configuration

### Custom Auto-Save Interval

```rust
// Save every 5 seconds instead of default 2 seconds
let plugin = WindowStatePlugin::new()
    .with_auto_save_interval(5);

plugins.install(plugin, Message::WindowState);
```

### Custom Window ID

```rust
// Track a specific window instead of the main window
let plugin = WindowStatePlugin::for_window(window::Id::unique());
plugins.install(plugin, Message::WindowState);
```

## Storage Location

The window state is saved as JSON in:
- **Linux**: `~/.config/iced_plugins/window_state.json`
- **macOS**: `~/Library/Application Support/iced_plugins/window_state.json`
- **Windows**: `%APPDATA%\iced_plugins\window_state.json`

### Example State File

```json
{
  "size": {
    "width": 1024.0,
    "height": 768.0
  },
  "position": {
    "x": 100.0,
    "y": 100.0
  },
  "maximized": false
}
```

## API Reference

### `WindowStatePlugin`

The main plugin struct.

- `new()` - Create with default settings
- `for_window(window::Id)` - Track a specific window
- `with_auto_save_interval(u64)` - Set auto-save interval in seconds
- `load_from_disk()` - Static method to load state before app creation

### `WindowState`

The window state data structure.

- `size: iced::Size` - Window dimensions
- `position: iced::Point` - Window position on screen
- `maximized: bool` - Maximized state
- `load()` - Load from disk
- `save()` - Save to disk
- `config_path()` - Get the config file path

### `WindowStateMessage`

Messages handled by the plugin.

- `WindowResized(iced::Size)` - Window was resized
- `WindowMoved(iced::Point)` - Window was moved
- `WindowMaximized(bool)` - Maximized state changed
- `SaveToDisk` - Trigger immediate save

### `WindowPluginState`

The state held by the plugin manager.

- `current_state()` - Get current window state
- `mark_dirty()` - Mark state as needing save
- `force_save()` - Immediately save to disk

## Manual Save

You can trigger a manual save:

```rust
// In your update method
Message::ManualSave => {
    if let Some(state) = self.plugins
        .get_typed_state_mut::<WindowPluginState>("window_state")
    {
        let _ = state.force_save();
    }
    Command::none()
}
```

## Example

See the [window_state_plugin example](../../examples/window_state_plugin.rs) for a complete working example.

Run it with:
```bash
cargo run --example window_state_plugin
```

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](../../LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](../../LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
