# Window State Plugin

Automatically save and restore window size and position in Iced apps.

## Example

```rust
use iced::{Element, Subscription, Task, window};
use iced::window::Position;
use iced_plugins::{PluginManager, PluginMessage};
use iced_window_state_plugin::WindowStatePlugin;

const APP_NAME: &str = "my_app";

fn main() -> iced::Result {
    // Load saved state before creating window
    let state = WindowStatePlugin::load(APP_NAME).unwrap_or_default();

    iced::application(App::new, App::update, App::view)
        .subscription(App::subscription)
        .window(window::Settings {
            size: state.size,
            position: Position::Specific(state.position),
            ..Default::default()
        })
        .run()
}

#[derive(Clone)]
enum Message {
    Plugin(PluginMessage),
}

impl From<PluginMessage> for Message {
    fn from(msg: PluginMessage) -> Self {
        Message::Plugin(msg)
    }
}

struct App {
    plugins: PluginManager,
}

impl App {
    fn new() -> (Self, Task<Message>) {
        let mut plugins = PluginManager::new();

        // Install plugin - it handles everything automatically!
        plugins.install(WindowStatePlugin::new(APP_NAME.to_string()));

        (App { plugins }, Task::none())
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Plugin(msg) => self.plugins.update(msg).map(From::from),
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        self.plugins.subscriptions().map(From::from)
    }
}
```

## How It Works

1. **Load** - Get saved state before app creation with `load(app_name)`
2. **Install** - Create plugin with your app name and install it
3. **Auto-Save** - Plugin automatically subscribes to window events (resize, move) and saves every 2 seconds

State is saved to:
- **Linux**: `~/.local/share/{app_name}/plugins/window_state.json`
- **macOS**: `~/Library/Application Support/{app_name}/plugins/window_state.json`
- **Windows**: `%LOCALAPPDATA%\{app_name}\plugins\window_state.json`

## Configuration

```rust
// Custom save interval (default: 2 seconds)
WindowStatePlugin::new("my_app".to_string()).with_auto_save_interval(5)
```

## Manual Operations

```rust
// Force immediate save
let handle = plugins.install(WindowStatePlugin::new("my_app".to_string()));
handle.dispatch(WindowStatePlugin::force_save())

// Reset to defaults
handle.dispatch(WindowStatePlugin::reset_to_default())

// Get current state and config path
let (state, path) = plugins
    .get_plugin_state::<WindowStatePlugin>()
    .map(|s| (s.current_state(), s.config_path()))
    .unwrap();
```

## Run Example

```bash
cargo run --example window_state_plugin
```
