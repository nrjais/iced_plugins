# Preference Store Plugin for Iced

A simple JSON-based preference store plugin for Iced applications that automatically persists data to disk.

## Features

- **Simple API**: Just get, set, and delete operations
- **Auto-save**: Changes are automatically persisted to disk
- **Group-based**: Organize preferences into logical groups
- **JSON storage**: Human-readable JSON files
- **Async operations**: Non-blocking file I/O

## Example

```rust
use iced_pref_store_plugin::{PrefStorePlugin, PrefMessage};
use iced_plugins::PluginManager;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UserPrefs {
    theme: String,
    font_size: u32,
}

const APP_NAME: &str = "my_app";

fn main() -> iced::Result {
    let mut builder = PluginManagerBuilder::new();

    // Install the preference store plugin
    let pref_handle = builder.install(PrefStorePlugin::new(APP_NAME));
    let (plugins, init_task) = builder.build();

    // Set a preference
    let prefs = UserPrefs {
        theme: "dark".to_string(),
        font_size: 14,
    };

    pref_handle.dispatch(PrefMessage::set("ui", "user", prefs));

    // Get a preference
    pref_handle.dispatch(PrefMessage::get("ui", "user"));

    // Delete a preference
    pref_handle.dispatch(PrefMessage::delete("ui", "user"));

    iced::application(App::new, App::update, App::view).run()
}
```

## Storage

Preferences are stored in JSON files organized by group:

```
~/.config/{app_name}/prefs/
  ├── ui.json
  ├── settings.json
  └── cache.json
```

## API

### Creating the Plugin

```rust
let mut builder = PluginManagerBuilder::new();
let pref_handle = builder.install(PrefStorePlugin::new("my_app"));
let (plugins, init_task) = builder.build();
```

### Messages

- **`PrefMessage::set(group, key, value)`** - Store a preference
  ```rust
  pref_handle.dispatch(PrefMessage::set("ui", "theme", "dark"));
  ```

- **`PrefMessage::get(group, key)`** - Retrieve a preference
  ```rust
  pref_handle.dispatch(PrefMessage::get("ui", "theme"));
  ```

- **`PrefMessage::delete(group, key)`** - Delete a preference
  ```rust
  pref_handle.dispatch(PrefMessage::delete("ui", "theme"));
  ```

### Outputs

Handle preference store outputs in your subscription:

```rust
fn subscription(app: &App) -> iced::Subscription<Message> {
    iced::Subscription::batch([
        app.plugins.subscriptions().map(Message::Plugin),
        app.pref_handle.listen().map(Message::PrefOutput),
    ])
}
```

Output variants:
- **`PrefOutput::Set { group, key }`** - A preference was stored
- **`PrefOutput::Get { group, key, value }`** - A preference was retrieved
- **`PrefOutput::NotFound { group, key }`** - A preference was not found
- **`PrefOutput::Deleted { group, key }`** - A preference was deleted
- **`PrefOutput::Error { message }`** - An error occurred

### Deserializing Values

Use the `as_value` helper to deserialize retrieved preferences:

```rust
match output {
    PrefOutput::Get { .. } => {
        if let Some(prefs) = output.as_value::<UserPrefs>() {
            // Use the deserialized preferences
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
