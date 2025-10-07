# Store Plugin for Iced

A simple JSON-based store plugin for Iced applications that automatically persists data to disk.

## Features

- Simple get, set, and delete operations
- Automatic persistence to disk
- Group-based organization
- Human-readable JSON storage
- Non-blocking async file I/O

## Usage

See [`examples/store_plugin.rs`](../../examples/store_plugin.rs) for a complete example.

Run the example with:
```sh
cargo run --example store_plugin
```

## Quick Start

```rust
use iced_store_plugin::{StorePlugin, StoreMessage};

// Create the plugin
let store_handle = builder.install(StorePlugin::new("my_app"));

// Set a value
store_handle.dispatch(StoreMessage::set("ui", "theme", "dark"));

// Get a value
store_handle.dispatch(StoreMessage::get("ui", "theme"));

// Delete a value
store_handle.dispatch(StoreMessage::delete("ui", "theme"));
```

## Storage Location

Data is stored in JSON files organized by group:
```
~/.config/{app_name}/store/
  ├── ui.json
  ├── settings.json
  └── cache.json
```
