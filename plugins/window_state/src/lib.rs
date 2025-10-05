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
//! - Manual save support
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
use iced::window::Event;
use iced::{Subscription, Task};
use iced_plugins::Plugin;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

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

/// Messages that the window state plugin handles
#[derive(Clone, Debug)]
pub enum WindowStateMessage {
    /// Window event
    WindowEvent(iced::Event),
    /// Trigger a save to disk
    SaveToDisk,
    /// Force save immediately
    ForceSave,
    /// Reset to default state
    ResetToDefault,
}

/// The plugin state held by the PluginManager
pub struct WindowPluginState {
    /// Current window state
    state: WindowState,
    /// Whether state has changed since last save
    dirty: bool,
    /// Config path
    config_path: PathBuf,
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
}

/// Window state plugin that manages window state persistence
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

    /// Create a message to force save the current state
    pub fn force_save() -> WindowStateMessage {
        WindowStateMessage::ForceSave
    }

    /// Create a message to reset state to default
    pub fn reset_to_default() -> WindowStateMessage {
        WindowStateMessage::ResetToDefault
    }

    fn config_path(app_name: &str) -> PathBuf {
        let config_dir = dirs::config_local_dir().unwrap_or_else(|| PathBuf::from("."));
        config_dir
            .join(app_name)
            .join("plugins")
            .join("window_state.json")
    }

    /// Load window state from disk
    pub fn load(app_name: &str) -> Option<WindowState> {
        let path = Self::config_path(app_name);
        let contents = fs::read_to_string(&path).ok()?;
        serde_json::from_str(&contents).ok()?
    }

    /// Save window state to disk
    fn save(&self, state: &WindowState) -> Result<(), Box<dyn std::error::Error>> {
        let path = &self.config_path;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let contents = serde_json::to_string_pretty(state)?;
        fs::write(path, contents)?;
        Ok(())
    }
}

impl Plugin for WindowStatePlugin {
    type Message = WindowStateMessage;
    type State = WindowPluginState;

    fn name(&self) -> &'static str {
        "window_state"
    }

    fn init(&self) -> Self::State {
        WindowPluginState {
            state: Self::load(&self.app_name).unwrap_or_default(),
            dirty: false,
            config_path: self.config_path.clone(),
        }
    }

    fn update(&self, state: &mut Self::State, message: Self::Message) -> Task<Self::Message> {
        match message {
            WindowStateMessage::WindowEvent(Window(Event::Resized(size))) => {
                if state.state.size != size {
                    state.state.size = size;
                    state.dirty = true;
                }
            }
            WindowStateMessage::WindowEvent(Window(Event::Moved(position))) => {
                if state.state.position != position {
                    state.state.position = position;
                    state.dirty = true;
                }
            }
            WindowStateMessage::SaveToDisk => {
                if state.dirty {
                    if let Err(e) = self.save(state.current_state()) {
                        eprintln!("Failed to save window state: {}", e);
                    } else {
                        state.dirty = false;
                    }
                }
            }
            WindowStateMessage::ForceSave => {
                if let Err(e) = self.save(state.current_state()) {
                    eprintln!("Failed to force save window state: {}", e);
                } else {
                    state.dirty = false;
                }
            }
            WindowStateMessage::ResetToDefault => {
                state.state = WindowState::default();
                state.dirty = true;
                if let Err(e) = self.save(state.current_state()) {
                    eprintln!("Failed to save reset window state: {}", e);
                } else {
                    state.dirty = false;
                }
            }
            WindowStateMessage::WindowEvent(_) => {
                // Ignore other window events, they are filtered out by the listen_with
            }
        }
        Task::none()
    }

    fn subscription(&self, _state: &Self::State) -> Subscription<Self::Message> {
        let window_event_sub = listen_with(|event, _, _| match &event {
            Window(Event::Moved(_) | Event::Resized(_)) => Some(event),
            _ => None,
        })
        .map(WindowStateMessage::WindowEvent);

        Subscription::batch([
            window_event_sub,
            every(Duration::from_secs(self.auto_save_interval))
                .map(|_| WindowStateMessage::SaveToDisk),
        ])
    }
}
