# Window State Plugin

Automatically save and restore window size and position in Iced applications.

## Features

- Automatically save window size and position
- Load saved state on app startup
- Configurable auto-save interval (default: 2 seconds)
- Manual save and reset operations

## Usage

See [`examples/window_state_plugin.rs`](../../examples/window_state_plugin.rs) for a complete example.

Run the example with:
```sh
cargo run --example window_state_plugin
```

## Quick Start

```rust
use iced_window_state_plugin::WindowStatePlugin;

// Load saved state before creating window
let state = WindowStatePlugin::load("my_app").unwrap_or_default();

iced::application(App::new, App::update, App::view)
    .window(window::Settings {
        size: state.size,
        position: Position::Specific(state.position),
        ..Default::default()
    })
    .run()

// Install plugin in your app
plugins.install(WindowStatePlugin::new("my_app".to_string()));
```

## Storage Location

State is saved to:
- **Linux**: `~/.local/share/{app_name}/plugins/window_state.json`
- **macOS**: `~/Library/Application Support/{app_name}/plugins/window_state.json`
- **Windows**: `%LOCALAPPDATA%\{app_name}\plugins\window_state.json`
