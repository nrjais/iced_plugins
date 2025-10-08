//! Window State Plugin for Iced
//!
//! This plugin automatically saves and restores window state (size, position)
//! to/from disk. It listens to window events and periodically saves changes.
//!
//! # Features
//!
//! - Automatic window state persistence per-application
//! - Load state before app creation
//! - Subscribe to window resize and move events
//! - Debounced auto-save every 2 seconds
//! - Only tracks the first window (main window) in multi-window apps
//!
//! # Example
//!
//! ```ignore
//! use iced_window_state_plugin::WindowStatePlugin;
//! use iced::window::Position;
//!
//! const APP_NAME: &str = "my_app";
//!
//! fn main() -> iced::Result {
//!     // Load window state before creating the app
//!     let window_state = WindowStatePlugin::load(APP_NAME).unwrap_or_default();
//!
//!     iced::application(App::new, App::update, App::view)
//!         .window(window::Settings {
//!             size: window_state.size,
//!             position: Position::Specific(window_state.position),
//!             ..Default::default()
//!         })
//!         .run()
//! }
//!
//! // In your app initialization:
//! let mut plugins = PluginManager::new();
//! plugins.install(WindowStatePlugin::new(APP_NAME.to_string()));
//! ```

use iced::Event::Window;
use iced::event::listen_with;
use iced::time::every;
use iced::window::{Event, Id};
use iced::{Subscription, Task};
use iced_plugins::Plugin;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;
use tokio::fs;

/// Window state data structure that is serialized to disk
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WindowState {
    /// Window size (width, height)
    #[serde(with = "size_serde")]
    pub size: iced::Size,
    /// Window position (x, y)
    #[serde(with = "point_serde")]
    pub position: iced::Point,
}

// Serde helpers for iced::Size
mod size_serde {
    use iced::Size;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    #[derive(Serialize, Deserialize)]
    struct SizeDef {
        width: f32,
        height: f32,
    }

    pub fn serialize<S>(size: &Size, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        SizeDef {
            width: size.width,
            height: size.height,
        }
        .serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Size, D::Error>
    where
        D: Deserializer<'de>,
    {
        let size_def = SizeDef::deserialize(deserializer)?;
        Ok(Size::new(size_def.width, size_def.height))
    }
}

// Serde helpers for iced::Point
mod point_serde {
    use iced::Point;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    #[derive(Serialize, Deserialize)]
    struct PointDef {
        x: f32,
        y: f32,
    }

    pub fn serialize<S>(point: &Point, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        PointDef {
            x: point.x,
            y: point.y,
        }
        .serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Point, D::Error>
    where
        D: Deserializer<'de>,
    {
        let point_def = PointDef::deserialize(deserializer)?;
        Ok(Point::new(point_def.x, point_def.y))
    }
}

impl Default for WindowState {
    fn default() -> Self {
        Self {
            size: iced::Size::new(800.0, 600.0),
            position: iced::Point::new(100.0, 100.0),
        }
    }
}

#[derive(Clone, Debug)]
pub enum WindowEvent {
    Resized(Id, iced::Size),
    Moved(Id, iced::Point),
    Opened(Id),
}

/// Messages that the window state plugin handles
#[derive(Clone, Debug)]
pub enum WindowStateMessage {
    /// Window event
    WindowEvent(WindowEvent),
    /// Trigger a save to disk
    SaveToDisk,
    /// Save operation completed
    SaveCompleted(Result<WindowState, String>),
}

/// Output messages emitted by the window state plugin
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum WindowStateOutput {
    /// Window state was saved to disk
    StateSaved(WindowState),
    /// Window state was updated (but not yet saved)
    StateUpdated(WindowState),
    /// An error occurred while saving
    SaveError(String),
    /// Window state was reset to default
    StateReset(WindowState),
}

/// The plugin state held by the PluginManager
#[derive(Debug, Clone)]
pub struct WindowPluginState {
    /// Current window state
    state: WindowState,
    /// Whether state has changed since last save
    dirty: bool,
    /// Config path
    config_path: PathBuf,
    /// The oldest (main) window ID that we track
    oldest_window_id: Option<Id>,
}

impl WindowPluginState {
    /// Get the current window state
    pub fn current_state(&self) -> &WindowState {
        &self.state
    }

    /// Get the config path
    pub fn config_path(&self) -> &PathBuf {
        &self.config_path
    }

    /// Get the oldest window ID being tracked
    pub fn oldest_window_id(&self) -> Option<Id> {
        self.oldest_window_id
    }
}

/// Window state plugin that manages window state persistence
#[derive(Debug, Clone)]
pub struct WindowStatePlugin {
    app_name: String,
    /// Auto-save interval in seconds
    auto_save_interval: u64,
    /// Config path
    config_path: PathBuf,
}

impl WindowStatePlugin {
    /// Create a new window state plugin with default settings (tracks main window)
    pub fn new(app_name: String) -> Self {
        let config_path = Self::config_path(&app_name);
        Self {
            app_name,
            auto_save_interval: 2,
            config_path,
        }
    }

    /// Set the auto-save interval in seconds
    pub fn with_auto_save_interval(mut self, seconds: u64) -> Self {
        self.auto_save_interval = seconds;
        self
    }

    fn config_path(app_name: &str) -> PathBuf {
        let config_dir = directories::BaseDirs::new()
            .map(|dirs| dirs.config_local_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));
        config_dir
            .join(app_name)
            .join("plugins")
            .join("window_state.json")
    }

    /// Load window state from disk (blocking version for pre-app initialization)
    pub fn load(app_name: &str) -> Option<WindowState> {
        let path = Self::config_path(app_name);
        let contents = std::fs::read_to_string(&path).ok()?;
        serde_json::from_str(&contents).ok()
    }

    /// Save window state to disk (async)
    async fn save_async(path: PathBuf, state: WindowState) -> Result<WindowState, String> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|e| format!("Failed to create config directory: {}", e))?;
        }
        let contents = serde_json::to_string_pretty(&state)
            .map_err(|e| format!("Failed to serialize window state: {}", e))?;
        fs::write(&path, contents)
            .await
            .map_err(|e| format!("Failed to write window state: {}", e))?;
        Ok(state)
    }
}

/// Subscription for listening to all window events
fn window_events() -> Subscription<WindowStateMessage> {
    listen_with(|event, _, id| match event {
        Window(Event::Moved(position)) => Some(WindowEvent::Moved(id, position)),
        Window(Event::Resized(size)) => Some(WindowEvent::Resized(id, size)),
        Window(Event::Opened { .. }) => Some(WindowEvent::Opened(id)),
        _ => None,
    })
    .map(WindowStateMessage::WindowEvent)
}

impl Plugin for WindowStatePlugin {
    type Message = WindowStateMessage;
    type State = WindowPluginState;
    type Output = WindowStateOutput;

    fn name(&self) -> &'static str {
        "window_state"
    }

    fn init(&self) -> (Self::State, Task<Self::Message>) {
        let state = WindowPluginState {
            state: Self::load(&self.app_name).unwrap_or_default(),
            dirty: false,
            config_path: self.config_path.clone(),
            oldest_window_id: None,
        };
        (state, Task::none())
    }

    fn update(
        &self,
        state: &mut Self::State,
        message: Self::Message,
    ) -> (Task<Self::Message>, Option<Self::Output>) {
        match message {
            WindowStateMessage::WindowEvent(WindowEvent::Opened(id)) => {
                if state.oldest_window_id.is_none() {
                    state.oldest_window_id = Some(id);
                }
                (Task::none(), None)
            }
            WindowStateMessage::WindowEvent(WindowEvent::Resized(id, size)) => {
                if state.oldest_window_id != Some(id) {
                    return (Task::none(), None);
                }

                if state.state.size != size {
                    state.state.size = size;
                    state.dirty = true;
                    (
                        Task::none(),
                        Some(WindowStateOutput::StateUpdated(state.state.clone())),
                    )
                } else {
                    (Task::none(), None)
                }
            }
            WindowStateMessage::WindowEvent(WindowEvent::Moved(id, position)) => {
                if state.oldest_window_id != Some(id) {
                    return (Task::none(), None);
                }

                if state.state.position != position {
                    state.state.position = position;
                    state.dirty = true;
                    (
                        Task::none(),
                        Some(WindowStateOutput::StateUpdated(state.state.clone())),
                    )
                } else {
                    (Task::none(), None)
                }
            }
            WindowStateMessage::SaveToDisk => {
                if state.dirty {
                    let path = state.config_path.clone();
                    let window_state = state.state.clone();
                    let task = Task::perform(
                        Self::save_async(path, window_state),
                        WindowStateMessage::SaveCompleted,
                    );
                    (task, None)
                } else {
                    (Task::none(), None)
                }
            }
            WindowStateMessage::SaveCompleted(result) => match result {
                Ok(saved_state) => {
                    state.dirty = false;
                    (
                        Task::none(),
                        Some(WindowStateOutput::StateSaved(saved_state)),
                    )
                }
                Err(e) => {
                    eprintln!("Failed to save window state: {}", e);
                    (Task::none(), Some(WindowStateOutput::SaveError(e)))
                }
            },
        }
    }

    fn subscription(&self, _state: &Self::State) -> Subscription<Self::Message> {
        Subscription::batch([
            window_events(),
            every(Duration::from_secs(self.auto_save_interval))
                .map(|_| WindowStateMessage::SaveToDisk),
        ])
    }
}
