//! Window State Plugin for Iced
//!
//! This plugin automatically saves and restores window state (size, position, maximized)
//! to/from disk. It listens to window events and periodically saves changes.
//!
//! # Features
//!
//! - Automatic window state persistence
//! - Load state before app creation
//! - Subscribe to window resize events
//! - Debounced auto-save every 2 seconds
//! - Manual save support
//!
//! # Example
//!
//! ```ignore
//! use iced_window_state_plugin::{WindowStatePlugin, WindowState};
//!
//! fn main() -> iced::Result {
//!     // Load window state before creating the app
//!     let window_state = WindowStatePlugin::load_from_disk();
//!
//!     let settings = Settings {
//!         window: window::Settings {
//!             size: window_state.size,
//!             position: window_state.position,
//!             ..Default::default()
//!         },
//!         ..Default::default()
//!     };
//!
//!     App::run(settings)
//! }
//! ```

use iced::Event::Window;
use iced::event::listen_with;
use iced::time::every;
use iced::window::Event;
use iced::{Subscription, Task, window};
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
    /// Whether the window is maximized
    pub maximized: bool,
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
            maximized: false,
        }
    }
}

impl WindowState {
    /// Get the path to the configuration file
    pub fn config_path() -> PathBuf {
        let config_dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        config_dir.join("iced_plugins").join("window_state.json")
    }

    /// Load window state from disk
    pub fn load() -> Self {
        let path = Self::config_path();
        if let Ok(contents) = fs::read_to_string(&path) {
            serde_json::from_str(&contents).unwrap_or_default()
        } else {
            Self::default()
        }
    }

    /// Save window state to disk
    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let contents = serde_json::to_string_pretty(self)?;
        fs::write(path, contents)?;
        Ok(())
    }
}

/// Messages that the window state plugin handles
#[derive(Clone, Debug)]
pub enum WindowStateMessage {
    /// Window event
    WindowEvent(iced::Event),
    /// Trigger a save to disk
    SaveToDisk,
}

/// The plugin state held by the PluginManager
pub struct WindowPluginState {
    /// Current window state
    pub state: WindowState,
    /// Whether state has changed since last save
    pub dirty: bool,
}

impl WindowPluginState {
    /// Get the current window state
    pub fn current_state(&self) -> &WindowState {
        &self.state
    }

    /// Mark state as dirty (needs saving)
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Force save the current state to disk
    pub fn force_save(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.state.save()?;
        self.dirty = false;
        Ok(())
    }
}

/// Window state plugin that manages window state persistence
pub struct WindowStatePlugin {
    /// Window ID to track
    pub window_id: Option<window::Id>,
    /// Auto-save interval in seconds
    pub auto_save_interval: u64,
}

impl Default for WindowStatePlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl WindowStatePlugin {
    /// Create a new window state plugin with default settings (tracks main window)
    pub fn new() -> Self {
        Self {
            window_id: None, // None means use the default/main window
            auto_save_interval: 2,
        }
    }

    /// Create a plugin for a specific window
    pub fn for_window(window_id: window::Id) -> Self {
        Self {
            window_id: Some(window_id),
            auto_save_interval: 2,
        }
    }

    /// Set the auto-save interval in seconds
    pub fn with_auto_save_interval(mut self, seconds: u64) -> Self {
        self.auto_save_interval = seconds;
        self
    }

    pub fn load_from_disk() -> WindowState {
        WindowState::load()
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
            state: WindowState::load(),
            dirty: false,
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
                    if let Err(e) = state.state.save() {
                        eprintln!("Failed to save window state: {}", e);
                    } else {
                        state.dirty = false;
                    }
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
