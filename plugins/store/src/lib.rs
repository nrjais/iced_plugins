//! Store Plugin for Iced
//!
//! A simple JSON-based store plugin that persists data to disk.
//! Each group is stored in a separate JSON file.
//!
//! # Features
//!
//! - Simple get/set/delete operations
//! - Group-based organization (separate files per group)
//! - Automatic persistence to disk
//! - In-memory caching for fast access
//! - Access data directly outside application
//! - Platform-specific storage locations
//!
//! # Usage
//!
//! There are two ways to use this plugin:
//!
//! ## 1. With the Plugin System (for Iced applications)
//!
//! ```ignore
//! use iced_store_plugin::{StorePlugin, StoreInput, StoreOutput, AppName};
//! use iced_plugins::PluginManagerBuilder;
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Debug, Clone, Serialize, Deserialize)]
//! struct UserPrefs {
//!     theme: String,
//!     font_size: u32,
//! }
//!
//! fn main() -> iced::Result {
//!     let mut builder = PluginManagerBuilder::new();
//!     let app_name = AppName::new("com", "mycompany", "myapp");
//!     let store_handle = builder.install(StorePlugin::new(app_name));
//!     let (plugins, init_task) = builder.build();
//!
//!     // Set a value
//!     let prefs = UserPrefs {
//!         theme: "dark".to_string(),
//!         font_size: 14,
//!     };
//!     store_handle.dispatch(StoreInput::set("ui", "prefs", prefs));
//!
//!     // Get a value
//!     store_handle.dispatch(StoreInput::get("ui", "prefs"));
//!
//!     // Handle the output in your update function
//!     // match output {
//!     //     StoreOutput::Get { value, .. } => {
//!     //         let prefs: UserPrefs = serde_json::from_str(&value).ok()?;
//!     //         // Use the prefs...
//!     //     }
//!     //     _ => {}
//!     // }
//!
//!     iced::application(App::new, App::update, App::view).run()
//! }
//! ```
//!
//! ## 2. Without the Plugin System (for CLI tools and scripts)
//!
//! ```ignore
//! use iced_store_plugin::{AppName, read_value, write_value};
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Debug, Clone, Serialize, Deserialize)]
//! struct UserPrefs {
//!     theme: String,
//!     font_size: u32,
//! }
//!
//! #[tokio::main]
//! async fn main() -> Result<(), String> {
//!     let app_name = AppName::new("com", "mycompany", "myapp");
//!
//!     // Write a value
//!     let prefs = UserPrefs {
//!         theme: "dark".to_string(),
//!         font_size: 14,
//!     };
//!     write_value(&app_name, "ui", "prefs", &prefs).await?;
//!
//!     // Read it back
//!     let loaded: UserPrefs = read_value(&app_name, "ui", "prefs").await?;
//!     println!("Theme: {}", loaded.theme);
//!
//!     Ok(())
//! }
//! ```
//! Each group is stored in a separate JSON file named `<group>.json`.

mod app_name;
mod helpers;
mod messages;
mod plugin;
mod storage;

// Re-export public API
pub use app_name::AppName;
pub use helpers::{delete_value, has_value, list_keys, read_value, write_value};
pub use messages::{StoreInput, StoreMessage, StoreOutput};
pub use plugin::{StorePlugin, StoreState};
pub use storage::{get_group_path, storage_dir};
