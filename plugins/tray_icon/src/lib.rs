//! Tray Icon Plugin for Iced
//!
//! This plugin provides system tray icon functionality for Iced applications.
//! ```

mod menu;

use iced::futures::SinkExt;
use iced::futures::channel::mpsc::Sender;
use iced::{Subscription, Task};
use iced_plugins::Plugin;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::Duration;

// Re-export only Icon for convenience
pub use tray_icon::Icon;

use tray_icon::menu::MenuEvent;
use tray_icon::{TrayIcon, TrayIconBuilder, TrayIconEvent};

pub use menu::{Menu, MenuItem};
use menu::{NativeMenuItem, update_menu_items};

use crate::menu::{build_native_menu, create_icon};

#[cfg(target_os = "linux")]
use gtk::glib;

/// Public input API that applications use
#[derive(Clone, Debug)]
pub enum TrayIconInput {
    /// Update the tray icon
    SetIcon(Vec<u8>),
    /// Update the tooltip
    SetTooltip(Option<String>),
    /// Update the menu
    UpdateMenu(Menu),
    /// Show the tray icon
    Show,
    /// Hide the tray icon
    Hide,
}

impl From<TrayIconInput> for TrayIconMessage {
    fn from(input: TrayIconInput) -> Self {
        match input {
            TrayIconInput::SetIcon(data) => TrayIconMessage::SetIcon(data),
            TrayIconInput::SetTooltip(tooltip) => TrayIconMessage::SetTooltip(tooltip),
            TrayIconInput::UpdateMenu(menu) => TrayIconMessage::UpdateMenu(menu),
            TrayIconInput::Show => TrayIconMessage::Show,
            TrayIconInput::Hide => TrayIconMessage::Hide,
        }
    }
}

/// Internal messages that the tray icon plugin handles
/// Note: This is for internal use. Applications should use `TrayIconInput` instead.
#[derive(Clone, Debug)]
pub enum TrayIconMessage {
    /// Update the tray icon
    SetIcon(Vec<u8>),
    /// Update the tooltip
    SetTooltip(Option<String>),
    /// Update the menu
    UpdateMenu(Menu),
    /// Menu event occurred
    MenuEvent(String),
    /// Tray icon event occurred
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
pub struct TrayIconState {
    /// The tray icon instance (wrapped for Send)
    tray_icon: Option<TrayIconWrapper>,
    /// Current tooltip
    tooltip: Option<String>,
    /// Current icon bytes
    icon_bytes: Option<Vec<u8>>,
    /// Current menu data
    current_menu: Option<Menu>,
    /// Native menu items lookup by ID
    native_items: HashMap<String, Arc<NativeMenuItem>>,
}

impl std::fmt::Debug for TrayIconState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TrayIconState")
            .field("has_tray_icon", &self.tray_icon.is_some())
            .field("tooltip", &self.tooltip)
            .field("has_icon_bytes", &self.icon_bytes.is_some())
            .field("has_menu", &self.current_menu.is_some())
            .field("native_items_count", &self.native_items.len())
            .finish()
    }
}

/// Tray icon plugin configuration
#[derive(Clone, Debug)]
pub struct TrayIconPlugin {
    /// Tooltip text for the tray icon
    tooltip: Option<String>,
    /// Icon data (PNG format)
    icon_data: Option<Vec<u8>>,
    /// Menu
    menu: Option<Menu>,
}

impl TrayIconPlugin {
    /// Create a new tray icon plugin with a tooltip
    pub fn new(tooltip: impl Into<String>) -> Self {
        Self {
            tooltip: Some(tooltip.into()),
            icon_data: None,
            menu: None,
        }
    }

    /// Set the icon from raw bytes (PNG format)
    pub fn with_icon(mut self, icon_data: Vec<u8>) -> Self {
        self.icon_data = Some(icon_data);
        self
    }

    /// Set the icon from a resource
    pub fn with_icon_from_slice(mut self, bytes: &[u8]) -> Self {
        self.icon_data = Some(bytes.to_vec());
        self
    }

    /// Set the menu
    pub fn with_menu(mut self, menu: Menu) -> Self {
        self.menu = Some(menu);
        self
    }

    /// Set the tooltip
    pub fn with_tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }
}

impl Plugin for TrayIconPlugin {
    type Input = TrayIconInput;
    type Message = TrayIconMessage;
    type State = TrayIconState;
    type Output = TrayIconOutput;

    fn name(&self) -> &'static str {
        "tray_icon"
    }

    fn init(&self) -> (Self::State, Task<Self::Message>) {
        // Create icon if data is provided
        let icon = if let Some(ref icon_data) = self.icon_data {
            match create_icon(icon_data) {
                Ok(icon) => Some(icon),
                Err(e) => {
                    eprintln!("Failed to create tray icon: {}", e);
                    None
                }
            }
        } else {
            None
        };

        // Build native menu if provided
        let (native_menu, native_items) = if let Some(ref menu) = self.menu {
            let (native, items) = build_native_menu(menu);
            (Some(native), items)
        } else {
            (None, HashMap::new())
        };

        // Initialize GTK and create tray icon
        let mut tray_icon = None;

        #[cfg(target_os = "linux")]
        {
            // Initialize GTK first
            println!("Running on Linux - Initializing GTK...");
            if let Err(e) = gtk::init() {
                eprintln!("Failed to initialize GTK: {}", e);
            } else {
                println!("GTK initialized successfully");

                // Create tray icon
                if let Some(icon) = icon {
                    println!("Creating tray icon with icon data...");
                    let mut builder = TrayIconBuilder::new();
                    builder = builder.with_icon(icon);

                    if let Some(ref tooltip) = self.tooltip {
                        println!("Setting tooltip: {}", tooltip);
                        builder = builder.with_tooltip(tooltip);
                    }

                    if let Some(native_menu) = native_menu {
                        println!("Setting menu...");
                        builder = builder.with_menu(Box::new(native_menu));
                    }

                    match builder.build() {
                        Ok(tray) => {
                            // Ensure the tray icon is visible
                            if let Err(e) = tray.set_visible(true) {
                                eprintln!("Failed to make tray icon visible: {}", e);
                            }

                            tray_icon = Some(TrayIconWrapper::new(tray));
                            println!("Tray icon created successfully and set to visible");

                            // Start GTK event loop in a background thread
                            std::thread::spawn(|| {
                                println!("Starting GTK event loop thread...");
                                loop {
                                    // Process all pending GTK events
                                    glib::MainContext::default().iteration(true);
                                }
                            });
                        }
                        Err(e) => {
                            eprintln!("Failed to build tray icon: {}", e);
                        }
                    }
                }
            }
        }

        #[cfg(not(target_os = "linux"))]
        {
            // For non-Linux platforms, create tray icon directly
            println!("Running on non-Linux platform - creating tray icon directly");
            if let Some(icon) = icon {
                let mut builder = TrayIconBuilder::new();
                builder = builder.with_icon(icon);

                if let Some(ref tooltip) = self.tooltip {
                    builder = builder.with_tooltip(tooltip);
                }

                if let Some(native_menu) = native_menu {
                    builder = builder.with_menu(Box::new(native_menu));
                }

                match builder.build() {
                    Ok(tray) => {
                        tray_icon = Some(TrayIconWrapper::new(tray));
                    }
                    Err(e) => {
                        eprintln!("Failed to build tray icon: {}", e);
                    }
                }
            }
        }

        let state = TrayIconState {
            tray_icon,
            tooltip: self.tooltip.clone(),
            icon_bytes: self.icon_data.clone(),
            current_menu: self.menu.clone(),
            native_items,
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
                    match create_icon(&bytes) {
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

            TrayIconMessage::UpdateMenu(new_menu) => {
                // Update existing native menu items with new data
                for item in new_menu.items() {
                    update_menu_items(item, &state.native_items);
                }

                state.current_menu = Some(new_menu);
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

                tokio::time::sleep(Duration::from_millis(10)).await;
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

                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        },
    ))
}
