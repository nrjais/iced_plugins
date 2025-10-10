//! Plugin implementation for the Iced framework

use crate::app_name::AppName;
use crate::messages::{StoreInput, StoreMessage, StoreOutput};
use crate::storage::{load_group, save_group};
use iced::{Subscription, Task};
use iced_plugins::Plugin;
use std::collections::HashMap;

/// The plugin state held by the PluginManager
///
/// This state maintains an in-memory cache of the store data for fast access.
#[derive(Debug)]
pub struct StoreState {
    /// In-memory store organized by group
    store: HashMap<String, HashMap<String, String>>,
    /// Application name for storage
    app_name: AppName,
}

/// Store plugin that manages persistent key-value storage
///
/// This plugin provides:
/// - In-memory caching for fast access
/// - Automatic persistence to disk
/// - Group-based organization
/// - JSON file storage
///
/// # Example
///
/// ```ignore
/// use iced_store_plugin::{StorePlugin, StoreInput, AppName};
/// use iced_plugins::PluginManagerBuilder;
///
/// fn setup_plugins() {
///     let mut builder = PluginManagerBuilder::new();
///     let app_name = AppName::new("com", "example", "myapp");
///     let store_handle = builder.install(StorePlugin::new(app_name));
///     let (plugins, init_task) = builder.build();
///
///     // Use the plugin
///     store_handle.dispatch(StoreInput::set("settings", "theme", "dark"));
/// }
/// ```
#[derive(Clone, Debug)]
pub struct StorePlugin {
    app_name: AppName,
}

impl StorePlugin {
    /// Create a new store plugin
    ///
    /// # Arguments
    ///
    /// * `app_name` - The application name used to determine storage location
    ///
    /// # Example
    ///
    /// ```
    /// use iced_store_plugin::{StorePlugin, AppName};
    ///
    /// let app_name = AppName::new("com", "example", "myapp");
    /// let plugin = StorePlugin::new(app_name);
    /// ```
    pub fn new(app_name: AppName) -> Self {
        Self { app_name }
    }
}

impl Plugin for StorePlugin {
    type Input = StoreInput;
    type Message = StoreMessage;
    type State = StoreState;
    type Output = StoreOutput;

    fn name(&self) -> &'static str {
        "store"
    }

    fn init(&self) -> (Self::State, Task<Self::Message>) {
        let state = StoreState {
            store: HashMap::new(),
            app_name: self.app_name.clone(),
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

                let app_name = state.app_name.clone();
                let data = state.store.get(&group).cloned().unwrap_or_default();
                let group_clone = group.clone();

                let task = Task::perform(
                    async move {
                        let success = save_group(&app_name, &group_clone, data).await.is_ok();
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

                let app_name = state.app_name.clone();
                let group_clone = group.clone();
                let key_clone = key.clone();

                let task = Task::perform(
                    async move {
                        let data = load_group(&app_name, &group_clone)
                            .await
                            .unwrap_or_default();
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
                if let Some(group_data) = state.store.get_mut(&group)
                    && group_data.remove(&key).is_some()
                {
                    let app_name = state.app_name.clone();
                    let data = group_data.clone();
                    let group_clone = group.clone();

                    let task = Task::perform(
                        async move {
                            let success = save_group(&app_name, &group_clone, data).await.is_ok();
                            StoreMessage::SaveResult {
                                group: group_clone,
                                success,
                            }
                        },
                        std::convert::identity,
                    );

                    return (task, Some(StoreOutput::Deleted { group, key }));
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
