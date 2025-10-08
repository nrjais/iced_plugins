//! Tray Icon Plugin for Iced
//!
//! This plugin provides system tray icon functionality for Iced applications.
//! ```

use iced::futures::SinkExt;
use iced::futures::channel::mpsc::Sender;
use iced::{Subscription, Task};
use iced_plugins::Plugin;
use std::sync::Arc;

// Re-export tray-icon types for direct access
pub use tray_icon;
pub use tray_icon::menu;
pub use tray_icon::{Icon, TrayIconBuilder};

use tray_icon::menu::{Menu, MenuEvent};
use tray_icon::{TrayIcon, TrayIconEvent};

/// Messages that the tray icon plugin handles
#[derive(Clone, Debug)]
pub enum TrayIconMessage {
    /// Update the tray icon
    SetIcon(Vec<u8>),
    /// Update the tooltip
    SetTooltip(Option<String>),
    /// Internal: menu event occurred
    MenuEvent(String),
    /// Internal: tray icon event occurred
    TrayEvent(TrayIconEventKind),
    /// Show the tray icon
    Show,
    /// Hide the tray icon
    Hide,
}

/// Tray icon events
#[derive(Clone, Debug)]
pub enum TrayIconEventKind {
    /// Left mouse button clicked
    Click,
    /// Double clicked
    DoubleClick,
}

/// Output messages emitted by the tray icon plugin
#[derive(Clone, Debug)]
pub enum TrayIconOutput {
    /// A menu item was clicked (returns the MenuId as a string)
    MenuItemClicked { id: String },
    /// The tray icon was clicked
    IconClicked,
    /// The tray icon was double-clicked
    IconDoubleClicked,
    /// An error occurred
    Error { message: String },
}

// Wrapper types to make TrayIcon Send
struct TrayIconWrapper(TrayIcon);

impl TrayIconWrapper {
    fn new(tray: TrayIcon) -> Self {
        Self(tray)
    }

    fn with_mut<F, R>(&mut self, f: F) -> R
    where
        F: FnOnce(&mut TrayIcon) -> R,
    {
        f(&mut self.0)
    }
}

impl std::fmt::Debug for TrayIconWrapper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TrayIconWrapper").finish()
    }
}

// SAFETY: We control access to TrayIcon through a Mutex
unsafe impl Send for TrayIconWrapper {}
unsafe impl Sync for TrayIconWrapper {}

/// The plugin state held by the PluginManager
#[derive(Debug)]
pub struct TrayIconState {
    /// The tray icon instance (wrapped for Send)
    tray_icon: Option<TrayIconWrapper>,
    /// Current tooltip
    tooltip: Option<String>,
    /// Current icon bytes
    icon_bytes: Option<Vec<u8>>,
}

/// Tray icon plugin configuration
#[derive(Clone)]
pub struct TrayIconPlugin {
    /// Tooltip text for the tray icon
    tooltip: Option<String>,
    /// Icon data (PNG format)
    icon_data: Option<Vec<u8>>,
    /// Menu (stored as a function that creates it, since Menu is not Clone)
    menu_fn: Option<Arc<dyn Fn() -> Menu + Send + Sync>>,
}

impl std::fmt::Debug for TrayIconPlugin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TrayIconPlugin")
            .field("tooltip", &self.tooltip)
            .field("has_icon_data", &self.icon_data.is_some())
            .field("has_menu", &self.menu_fn.is_some())
            .finish()
    }
}

impl TrayIconPlugin {
    /// Create a new tray icon plugin with a tooltip
    pub fn new(tooltip: impl Into<String>) -> Self {
        Self {
            tooltip: Some(tooltip.into()),
            icon_data: None,
            menu_fn: None,
        }
    }

    /// Set the icon from raw bytes (PNG format)
    pub fn with_icon(mut self, icon_data: Vec<u8>) -> Self {
        self.icon_data = Some(icon_data);
        self
    }

    /// Set the icon from a resource
    pub fn with_icon_from_resource(mut self, bytes: &[u8]) -> Self {
        self.icon_data = Some(bytes.to_vec());
        self
    }

    /// Set the menu using a builder function
    /// The function will be called during initialization to create the menu.
    ///
    /// # Example
    /// ```ignore
    /// TrayIconPlugin::new("My App")
    ///     .with_menu(|| {
    ///         let menu = menu::Menu::new();
    ///         menu.append(&menu::MenuItem::new("Item", true, None)).unwrap();
    ///         menu
    ///     })
    /// ```
    pub fn with_menu<F>(mut self, menu_builder: F) -> Self
    where
        F: Fn() -> Menu + Send + Sync + 'static,
    {
        self.menu_fn = Some(Arc::new(menu_builder));
        self
    }

    /// Set the tooltip
    pub fn with_tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }

    /// Create an icon from bytes
    fn create_icon(bytes: &[u8]) -> Result<Icon, String> {
        let image = image::load_from_memory(bytes)
            .map_err(|e| format!("Failed to load icon image: {}", e))?;

        let rgba = image.to_rgba8();
        let (width, height) = rgba.dimensions();

        Icon::from_rgba(rgba.into_raw(), width, height)
            .map_err(|e| format!("Failed to create icon: {}", e))
    }
}

impl Plugin for TrayIconPlugin {
    type Message = TrayIconMessage;
    type State = TrayIconState;
    type Output = TrayIconOutput;

    fn name(&self) -> &'static str {
        "tray_icon"
    }

    fn init(&self) -> (Self::State, Task<Self::Message>) {
        // Create icon if data is provided
        let icon = if let Some(ref icon_data) = self.icon_data {
            match Self::create_icon(icon_data) {
                Ok(icon) => Some(icon),
                Err(e) => {
                    eprintln!("Failed to create tray icon: {}", e);
                    None
                }
            }
        } else {
            None
        };

        // Build tray icon
        let tray_icon = if icon.is_some() {
            let mut builder = TrayIconBuilder::new();

            if let Some(icon) = icon {
                builder = builder.with_icon(icon);
            }

            if let Some(ref tooltip) = self.tooltip {
                builder = builder.with_tooltip(tooltip);
            }

            if let Some(ref menu_fn) = self.menu_fn {
                let menu = menu_fn();
                builder = builder.with_menu(Box::new(menu));
            }

            match builder.build() {
                Ok(tray) => Some(TrayIconWrapper::new(tray)),
                Err(e) => {
                    eprintln!("Failed to build tray icon: {}", e);
                    None
                }
            }
        } else {
            None
        };

        let state = TrayIconState {
            tray_icon,
            tooltip: self.tooltip.clone(),
            icon_bytes: self.icon_data.clone(),
        };

        (state, Task::none())
    }

    fn update(
        &self,
        state: &mut Self::State,
        message: Self::Message,
    ) -> (Task<Self::Message>, Option<Self::Output>) {
        match message {
            TrayIconMessage::SetIcon(bytes) => {
                if let Some(tray_wrapper) = state.tray_icon.as_mut() {
                    match Self::create_icon(&bytes) {
                        Ok(icon) => {
                            let result = tray_wrapper.with_mut(|tray| tray.set_icon(Some(icon)));
                            if let Err(e) = result {
                                return (
                                    Task::none(),
                                    Some(TrayIconOutput::Error {
                                        message: format!("Failed to set icon: {}", e),
                                    }),
                                );
                            }
                            state.icon_bytes = Some(bytes);
                        }
                        Err(e) => {
                            return (Task::none(), Some(TrayIconOutput::Error { message: e }));
                        }
                    }
                }
                (Task::none(), None)
            }

            TrayIconMessage::SetTooltip(tooltip) => {
                if let Some(tray_wrapper) = state.tray_icon.as_mut() {
                    let result = tray_wrapper.with_mut(|tray| tray.set_tooltip(tooltip.clone()));
                    if let Err(e) = result {
                        return (
                            Task::none(),
                            Some(TrayIconOutput::Error {
                                message: format!("Failed to set tooltip: {}", e),
                            }),
                        );
                    }
                    state.tooltip = tooltip;
                }
                (Task::none(), None)
            }

            TrayIconMessage::MenuEvent(id) => {
                (Task::none(), Some(TrayIconOutput::MenuItemClicked { id }))
            }

            TrayIconMessage::TrayEvent(kind) => {
                let output = match kind {
                    TrayIconEventKind::Click => TrayIconOutput::IconClicked,
                    TrayIconEventKind::DoubleClick => TrayIconOutput::IconDoubleClicked,
                };
                (Task::none(), Some(output))
            }

            TrayIconMessage::Show => {
                if let Some(tray_wrapper) = state.tray_icon.as_mut() {
                    let result = tray_wrapper.with_mut(|tray| tray.set_visible(true));
                    if let Err(e) = result {
                        return (
                            Task::none(),
                            Some(TrayIconOutput::Error {
                                message: format!("Failed to show tray icon: {}", e),
                            }),
                        );
                    }
                }
                (Task::none(), None)
            }

            TrayIconMessage::Hide => {
                if let Some(tray_wrapper) = state.tray_icon.as_mut() {
                    let result = tray_wrapper.with_mut(|tray| tray.set_visible(false));
                    if let Err(e) = result {
                        return (
                            Task::none(),
                            Some(TrayIconOutput::Error {
                                message: format!("Failed to hide tray icon: {}", e),
                            }),
                        );
                    }
                }
                (Task::none(), None)
            }
        }
    }

    fn subscription(&self, _state: &Self::State) -> Subscription<Self::Message> {
        let menu_sub = Subscription::run(menu_event_stream);
        let tray_sub = Subscription::run(tray_event_stream);

        Subscription::batch([menu_sub, tray_sub])
    }
}

/// Subscription for menu events
fn menu_event_stream() -> iced::futures::stream::BoxStream<'static, TrayIconMessage> {
    Box::pin(iced::stream::channel(
        100,
        |mut output: Sender<TrayIconMessage>| async move {
            let menu_channel = MenuEvent::receiver();

            loop {
                if let Ok(event) = menu_channel.try_recv() {
                    let _ = output.send(TrayIconMessage::MenuEvent(event.id.0)).await;
                }

                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            }
        },
    ))
}

/// Subscription for tray icon events
fn tray_event_stream() -> iced::futures::stream::BoxStream<'static, TrayIconMessage> {
    Box::pin(iced::stream::channel(
        100,
        |mut output: Sender<TrayIconMessage>| async move {
            let tray_channel = TrayIconEvent::receiver();

            loop {
                if let Ok(event) = tray_channel.try_recv() {
                    let kind = match event {
                        TrayIconEvent::Click { .. } => TrayIconEventKind::Click,
                        TrayIconEvent::DoubleClick { .. } => TrayIconEventKind::DoubleClick,
                        _ => continue,
                    };
                    let _ = output.send(TrayIconMessage::TrayEvent(kind)).await;
                }

                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            }
        },
    ))
}
