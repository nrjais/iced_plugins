# Store Plugin for Iced

A simple JSON-based store plugin for Iced applications that automatically persists data to disk.

## Features

- **Simple API**: Just get, set, and delete operations
- **Auto-save**: Changes are automatically persisted to disk
- **Group-based**: Organize data into logical groups
- **JSON storage**: Human-readable JSON files
- **Async operations**: Non-blocking file I/O

## Example

```rust
use iced_store_plugin::{StorePlugin, StoreMessage};
use iced_plugins::PluginManagerBuilder;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UserPrefs {
    theme: String,
    font_size: u32,
}

const APP_NAME: &str = "my_app";

fn main() -> iced::Result {
    let mut builder = PluginManagerBuilder::new();

    // Install the store plugin
    let store_handle = builder.install(StorePlugin::new(APP_NAME));
    let (plugins, init_task) = builder.build();

    // Set a value
    let prefs = UserPrefs {
        theme: "dark".to_string(),
        font_size: 14,
    };

    store_handle.dispatch(StoreMessage::set("ui", "user", prefs));

    // Get a value
    store_handle.dispatch(StoreMessage::get("ui", "user"));

    // Delete a value
    store_handle.dispatch(StoreMessage::delete("ui", "user"));

    iced::application(App::new, App::update, App::view).run()
}
```

## Storage

Data is stored in JSON files organized by group:

```
~/.config/{app_name}/store/
  ├── ui.json
  ├── settings.json
  └── cache.json
```

## API

### Creating the Plugin

```rust
let mut builder = PluginManagerBuilder::new();
let store_handle = builder.install(StorePlugin::new("my_app"));
let (plugins, init_task) = builder.build();
```

### Messages

- **`StoreMessage::set(group, key, value)`** - Store a value
  ```rust
  store_handle.dispatch(StoreMessage::set("ui", "theme", "dark"));
  ```

- **`StoreMessage::get(group, key)`** - Retrieve a value
  ```rust
  store_handle.dispatch(StoreMessage::get("ui", "theme"));
  ```

- **`StoreMessage::delete(group, key)`** - Delete a value
  ```rust
  store_handle.dispatch(StoreMessage::delete("ui", "theme"));
  ```

### Outputs

Handle store outputs in your subscription:

```rust
fn subscription(app: &App) -> iced::Subscription<Message> {
    iced::Subscription::batch([
        app.plugins.subscriptions().map(Message::Plugin),
        app.store_handle.listen().map(Message::StoreOutput),
    ])
}
```

Output variants:
- **`StoreOutput::Set { group, key }`** - A value was stored
- **`StoreOutput::Get { group, key, value }`** - A value was retrieved
- **`StoreOutput::NotFound { group, key }`** - A value was not found
- **`StoreOutput::Deleted { group, key }`** - A value was deleted
- **`StoreOutput::Error { message }`** - An error occurred

### Deserializing Values

Use the `as_value` helper to deserialize retrieved values:

```rust
match output {
    StoreOutput::Get { .. } => {
        if let Some(prefs) = output.as_value::<UserPrefs>() {
            // Use the deserialized value
            println!("Theme: {}", prefs.theme);
        }
    }
    _ => {}
}
```

## Design Philosophy

This plugin is intentionally kept simple:

- **Automatic persistence**: No manual save operations needed
- **Focused API**: Only essential operations (get, set, delete)
- **JSON-based**: Human-readable, debuggable storage format
- **No complex operations**: No listing, clearing, or batch operations

For more complex use cases, consider building on top of this plugin or using a dedicated database.
