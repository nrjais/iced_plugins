//! Store Plugin for Iced
//!
//! A simple JSON-based store plugin that persists data to disk.
//! Each group is stored in a separate JSON file.
//!
//! # Features
//!
//! - Simple get/set/delete operations
//! - Group-based organization
//! - Automatic persistence to disk
//! - Async file operations
//!
//! # Example
//!
//! ```ignore
//! use iced_store_plugin::{StorePlugin, StoreMessage};
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Debug, Clone, Serialize, Deserialize)]
//! struct UserPrefs {
//!     theme: String,
//!     font_size: u32,
//! }
//!
//! const APP_NAME: &str = "my_app";
//!
//! fn main() -> iced::Result {
//!     let mut builder = PluginManagerBuilder::new();
//!     let store_handle = builder.install(StorePlugin::new(APP_NAME));
//!     let (plugins, init_task) = builder.build();
//!
//!     // Set a value
//!     let prefs = UserPrefs {
//!         theme: "dark".to_string(),
//!         font_size: 14,
//!     };
//!     store_handle.dispatch(StoreMessage::set("ui", "user", prefs));
//!
//!     // Get a value
//!     store_handle.dispatch(StoreMessage::get("ui", "user"));
//!
//!     iced::application(App::new, App::update, App::view).run()
//! }
//! ```

use iced::{Subscription, Task};
use iced_plugins::Plugin;
use serde::{Serialize, de::DeserializeOwned};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::fs;

/// Messages that the store plugin handles
#[derive(Clone, Debug)]
pub enum StoreMessage {
    /// Set a value
    Set {
        group: String,
        key: String,
        value: String,
    },
    /// Get a value
    Get { group: String, key: String },
    /// Delete a value
    Delete { group: String, key: String },
    /// Internal: result of a get operation
    GetResult {
        group: String,
        key: String,
        value: Option<String>,
    },
    /// Internal: result of save operation
    SaveResult { group: String, success: bool },
}

impl StoreMessage {
    /// Create a Set message with serialization
    pub fn set<T>(group: impl Into<String>, key: impl Into<String>, value: T) -> Self
    where
        T: Serialize,
    {
        let value = serde_json::to_string(&value).unwrap_or_else(|e| {
            eprintln!("Failed to serialize value: {}", e);
            String::new()
        });

        Self::Set {
            group: group.into(),
            key: key.into(),
            value,
        }
    }

    /// Create a Get message
    pub fn get(group: impl Into<String>, key: impl Into<String>) -> Self {
        Self::Get {
            group: group.into(),
            key: key.into(),
        }
    }

    /// Create a Delete message
    pub fn delete(group: impl Into<String>, key: impl Into<String>) -> Self {
        Self::Delete {
            group: group.into(),
            key: key.into(),
        }
    }
}

/// Output messages emitted by the store plugin
#[derive(Clone, Debug)]
pub enum StoreOutput {
    /// A value was set
    Set { group: String, key: String },
    /// A value was retrieved
    Get {
        group: String,
        key: String,
        value: String,
    },
    /// A value was not found
    NotFound { group: String, key: String },
    /// A value was deleted
    Deleted { group: String, key: String },
    /// An error occurred
    Error { message: String },
}

impl StoreOutput {
    /// Try to deserialize a retrieved value
    pub fn as_value<T: DeserializeOwned>(&self) -> Option<T> {
        match self {
            StoreOutput::Get { value, .. } => serde_json::from_str(value).ok(),
            _ => None,
        }
    }
}

/// The plugin state held by the PluginManager
#[derive(Debug)]
pub struct StoreState {
    /// In-memory store organized by group
    store: HashMap<String, HashMap<String, String>>,
    /// Base directory for storage
    storage_dir: PathBuf,
}

impl StoreState {
    /// Get the storage path for a group
    fn group_path(&self, group: &str) -> PathBuf {
        self.storage_dir.join(format!("{}.json", group))
    }
}

/// Store plugin that manages persistent key-value storage
#[derive(Clone, Debug)]
pub struct StorePlugin {
    storage_dir: PathBuf,
}

impl StorePlugin {
    /// Create a new store plugin
    pub fn new(app_name: impl Into<String>) -> Self {
        let storage_dir = Self::storage_dir(&app_name.into());
        Self { storage_dir }
    }

    /// Get the storage directory path
    fn storage_dir(app_name: &str) -> PathBuf {
        directories::BaseDirs::new()
            .map(|dirs| dirs.config_local_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."))
            .join(app_name)
            .join("store")
    }

    /// Load a group from disk
    async fn load_group(path: PathBuf) -> Result<HashMap<String, String>, String> {
        if !path.exists() {
            return Ok(HashMap::new());
        }

        let contents = fs::read_to_string(&path)
            .await
            .map_err(|e| format!("Failed to read group file: {}", e))?;

        if contents.is_empty() {
            return Ok(HashMap::new());
        }

        serde_json::from_str(&contents).map_err(|e| format!("Failed to parse group file: {}", e))
    }

    /// Save a group to disk
    async fn save_group(path: PathBuf, data: HashMap<String, String>) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|e| format!("Failed to create storage directory: {}", e))?;
        }

        let contents = serde_json::to_string_pretty(&data)
            .map_err(|e| format!("Failed to serialize group: {}", e))?;

        fs::write(&path, contents)
            .await
            .map_err(|e| format!("Failed to write group file: {}", e))?;

        Ok(())
    }
}

impl Plugin for StorePlugin {
    type Message = StoreMessage;
    type State = StoreState;
    type Output = StoreOutput;

    fn name(&self) -> &'static str {
        "store"
    }

    fn init(&self) -> (Self::State, Task<Self::Message>) {
        let state = StoreState {
            store: HashMap::new(),
            storage_dir: self.storage_dir.clone(),
        };
        (state, Task::none())
    }

    fn update(
        &self,
        state: &mut Self::State,
        message: Self::Message,
    ) -> (Task<Self::Message>, Option<Self::Output>) {
        match message {
            StoreMessage::Set { group, key, value } => {
                state
                    .store
                    .entry(group.clone())
                    .or_insert_with(HashMap::new)
                    .insert(key.clone(), value);

                let path = state.group_path(&group);
                let data = state.store.get(&group).cloned().unwrap_or_default();
                let group_clone = group.clone();

                let task = Task::perform(
                    async move {
                        let success = Self::save_group(path, data).await.is_ok();
                        StoreMessage::SaveResult {
                            group: group_clone,
                            success,
                        }
                    },
                    std::convert::identity,
                );

                (task, Some(StoreOutput::Set { group, key }))
            }

            StoreMessage::Get { group, key } => {
                if let Some(group_data) = state.store.get(&group) {
                    if let Some(value) = group_data.get(&key) {
                        return (
                            Task::none(),
                            Some(StoreOutput::Get {
                                group,
                                key,
                                value: value.clone(),
                            }),
                        );
                    } else {
                        return (Task::none(), Some(StoreOutput::NotFound { group, key }));
                    }
                }

                let path = state.group_path(&group);
                let group_clone = group.clone();
                let key_clone = key.clone();

                let task = Task::perform(
                    async move {
                        let data = Self::load_group(path).await.unwrap_or_default();
                        let value = data.get(&key_clone).cloned();
                        StoreMessage::GetResult {
                            group: group_clone,
                            key: key_clone,
                            value,
                        }
                    },
                    std::convert::identity,
                );

                (task, None)
            }

            StoreMessage::GetResult { group, key, value } => {
                if let Some(ref json) = value {
                    state
                        .store
                        .entry(group.clone())
                        .or_insert_with(HashMap::new)
                        .insert(key.clone(), json.clone());
                }

                let output = if let Some(value) = value {
                    StoreOutput::Get { group, key, value }
                } else {
                    StoreOutput::NotFound { group, key }
                };

                (Task::none(), Some(output))
            }

            StoreMessage::Delete { group, key } => {
                if let Some(group_data) = state.store.get_mut(&group) {
                    if group_data.remove(&key).is_some() {
                        let data = group_data.clone();
                        let path = state.group_path(&group);
                        let group_clone = group.clone();

                        let task = Task::perform(
                            async move {
                                let success = Self::save_group(path, data).await.is_ok();
                                StoreMessage::SaveResult {
                                    group: group_clone,
                                    success,
                                }
                            },
                            std::convert::identity,
                        );

                        return (task, Some(StoreOutput::Deleted { group, key }));
                    }
                }

                (Task::none(), Some(StoreOutput::NotFound { group, key }))
            }

            StoreMessage::SaveResult { group, success } => {
                if !success {
                    return (
                        Task::none(),
                        Some(StoreOutput::Error {
                            message: format!("Failed to save group: {}", group),
                        }),
                    );
                }
                (Task::none(), None)
            }
        }
    }

    fn subscription(&self, _state: &Self::State) -> Subscription<Self::Message> {
        Subscription::none()
    }
}
