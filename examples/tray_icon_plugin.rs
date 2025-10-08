use iced::widget::{button, checkbox, column, row, scrollable, text};
use iced::{Element, Subscription, Task, window};
use iced_plugins::{PluginHandle, PluginManager, PluginManagerBuilder, PluginMessage};
use iced_tray_icon_plugin::{TrayIconMessage, TrayIconOutput, TrayIconPlugin, menu};

fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .subscription(App::subscription)
        .window(window::Settings {
            size: iced::Size::new(500.0, 600.0),
            ..Default::default()
        })
        .run()
}

#[derive(Clone, Debug)]
enum Message {
    Plugin(PluginMessage),
    TrayOutput(TrayIconOutput),
    ToggleVisibility,
    ToggleAutoStart,
    ToggleNotifications,
    ChangeStatus(Status),
    UpdateTrayIcon,
    UpdateTrayTooltip,
    UpdateTrayMenu,
    Quit,
}

impl From<PluginMessage> for Message {
    fn from(msg: PluginMessage) -> Self {
        Message::Plugin(msg)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Status {
    Online,
    Away,
    Busy,
    Offline,
}

impl Status {
    fn as_str(&self) -> &str {
        match self {
            Status::Online => "Online",
            Status::Away => "Away",
            Status::Busy => "Busy",
            Status::Offline => "Offline",
        }
    }

    fn color(&self) -> [u8; 3] {
        match self {
            Status::Online => [100, 255, 100],  // Green
            Status::Away => [255, 255, 100],    // Yellow
            Status::Busy => [255, 100, 100],    // Red
            Status::Offline => [128, 128, 128], // Gray
        }
    }
}

struct App {
    plugins: PluginManager,
    tray_handle: PluginHandle<TrayIconPlugin>,
    visible: bool,
    auto_start: bool,
    notifications_enabled: bool,
    status: Status,
    click_count: u32,
    last_menu_item: Option<String>,
}

impl App {
    fn new() -> (Self, Task<Message>) {
        // Create the icon
        let icon_data = create_icon(Status::Online.color());

        // Setup plugins with menu builder function
        let auto_start_init = true;
        let notifications_init = true;
        let status_init = Status::Online;

        let mut builder = PluginManagerBuilder::new();
        let tray_handle = builder.install(
            TrayIconPlugin::new("Tray Icon Demo")
                .with_icon(icon_data)
                .with_menu(move || {
                    Self::build_menu(auto_start_init, notifications_init, status_init)
                }),
        );

        let (plugins, init_task) = builder.build();

        (
            App {
                plugins,
                tray_handle,
                visible: true,
                auto_start: false,
                notifications_enabled: true,
                status: Status::Online,
                click_count: 0,
                last_menu_item: None,
            },
            init_task.map(From::from),
        )
    }

    fn build_menu(auto_start: bool, notifications: bool, status: Status) -> menu::Menu {
        use menu::{CheckMenuItem, Menu, MenuId, MenuItem, PredefinedMenuItem, Submenu};

        let menu = Menu::new();

        menu.append(&MenuItem::with_id(
            MenuId::new("show"),
            "Show Tray Icon",
            true,
            None,
        ))
        .unwrap();
        menu.append(&MenuItem::with_id(
            MenuId::new("hide"),
            "Hide Tray Icon",
            true,
            None,
        ))
        .unwrap();

        menu.append(&PredefinedMenuItem::separator()).unwrap();

        let status_menu = Submenu::with_id(MenuId::new("status_submenu"), "Status", true);
        status_menu
            .append(&CheckMenuItem::with_id(
                MenuId::new("status_online"),
                "🟢 Online",
                true,
                status == Status::Online,
                None,
            ))
            .unwrap();
        status_menu
            .append(&CheckMenuItem::with_id(
                MenuId::new("status_away"),
                "🟡 Away",
                true,
                status == Status::Away,
                None,
            ))
            .unwrap();
        status_menu
            .append(&CheckMenuItem::with_id(
                MenuId::new("status_busy"),
                "🔴 Busy",
                true,
                status == Status::Busy,
                None,
            ))
            .unwrap();
        status_menu
            .append(&CheckMenuItem::with_id(
                MenuId::new("status_offline"),
                "⚫ Offline",
                true,
                status == Status::Offline,
                None,
            ))
            .unwrap();

        menu.append(&status_menu).unwrap();
        menu.append(&PredefinedMenuItem::separator()).unwrap();

        // Settings submenu
        let settings_menu = Submenu::with_id(MenuId::new("settings_submenu"), "Settings", true);
        settings_menu
            .append(&CheckMenuItem::with_id(
                MenuId::new("auto_start"),
                "Start on Login",
                true,
                auto_start,
                None,
            ))
            .unwrap();
        settings_menu
            .append(&CheckMenuItem::with_id(
                MenuId::new("notifications"),
                "Enable Notifications",
                true,
                notifications,
                None,
            ))
            .unwrap();
        settings_menu
            .append(&PredefinedMenuItem::separator())
            .unwrap();
        settings_menu
            .append(&MenuItem::with_id(
                MenuId::new("preferences"),
                "Preferences...",
                true,
                None,
            ))
            .unwrap();

        menu.append(&settings_menu).unwrap();
        menu.append(&PredefinedMenuItem::separator()).unwrap();

        // About and Quit
        menu.append(&MenuItem::with_id(
            MenuId::new("about"),
            "About",
            true,
            None,
        ))
        .unwrap();
        menu.append(&PredefinedMenuItem::separator()).unwrap();
        menu.append(&MenuItem::with_id(MenuId::new("quit"), "Quit", true, None))
            .unwrap();

        menu
    }

    fn update_tray_menu(&self) -> Task<Message> {
        // Note: Dynamic menu rebuilding is not supported due to the tray-icon library
        // using non-Send types (Rc). To update menu state, you would need to:
        // 1. Store references to menu items during initialization
        // 2. Update them directly using their methods (e.g., CheckMenuItem::set_checked())
        //
        // For this example, we'll just log that the menu would be updated

        println!(
            "Menu state changed (auto_start: {}, notifications: {})",
            self.auto_start, self.notifications_enabled
        );
        Task::none()
    }

    fn update_tray_icon(&self) -> Task<Message> {
        let icon_data = create_icon(self.status.color());
        self.tray_handle
            .dispatch(TrayIconMessage::SetIcon(icon_data))
            .map(From::from)
    }

    fn update_tray_tooltip(&self) -> Task<Message> {
        let tooltip = format!(
            "Tray Demo - {} - Clicks: {}",
            self.status.as_str(),
            self.click_count
        );
        self.tray_handle
            .dispatch(TrayIconMessage::SetTooltip(Some(tooltip)))
            .map(From::from)
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Plugin(plugin_msg) => self.plugins.update(plugin_msg).map(From::from),

            Message::TrayOutput(output) => {
                match output {
                    TrayIconOutput::MenuItemClicked { id } => {
                        println!("Menu item clicked: {}", id);
                        self.last_menu_item = Some(id.clone());

                        match id.as_str() {
                            "show" => return self.update(Message::ToggleVisibility),
                            "hide" => return self.update(Message::ToggleVisibility),
                            "auto_start" => return self.update(Message::ToggleAutoStart),
                            "notifications" => return self.update(Message::ToggleNotifications),
                            "status_online" => {
                                return self.update(Message::ChangeStatus(Status::Online));
                            }
                            "status_away" => {
                                return self.update(Message::ChangeStatus(Status::Away));
                            }
                            "status_busy" => {
                                return self.update(Message::ChangeStatus(Status::Busy));
                            }
                            "status_offline" => {
                                return self.update(Message::ChangeStatus(Status::Offline));
                            }
                            "preferences" => {
                                println!("Opening preferences...");
                            }
                            "about" => {
                                println!(
                                    "About: Tray Icon Plugin Example\nDemonstrates dynamic menus, icons, and tooltips"
                                );
                            }
                            "quit" => return self.update(Message::Quit),
                            _ => {
                                println!("Unknown menu item: {}", id);
                            }
                        }
                    }
                    TrayIconOutput::IconClicked => {
                        println!("Tray icon clicked!");
                        self.click_count += 1;
                        return self.update_tray_tooltip();
                    }
                    TrayIconOutput::IconDoubleClicked => {
                        println!("Tray icon double-clicked!");
                        self.visible = true;
                    }
                    TrayIconOutput::Error { message } => {
                        eprintln!("Tray icon error: {}", message);
                    }
                }
                Task::none()
            }

            Message::ToggleVisibility => {
                self.visible = !self.visible;
                println!("Tray icon visibility: {}", self.visible);
                if self.visible {
                    self.tray_handle
                        .dispatch(TrayIconMessage::Show)
                        .map(From::from)
                } else {
                    self.tray_handle
                        .dispatch(TrayIconMessage::Hide)
                        .map(From::from)
                }
            }

            Message::ToggleAutoStart => {
                self.auto_start = !self.auto_start;
                println!("Auto-start: {}", self.auto_start);
                self.update_tray_menu()
            }

            Message::ToggleNotifications => {
                self.notifications_enabled = !self.notifications_enabled;
                println!("Notifications: {}", self.notifications_enabled);
                self.update_tray_menu()
            }

            Message::ChangeStatus(status) => {
                self.status = status;
                println!("Status changed to: {}", status.as_str());
                Task::batch([
                    self.update_tray_menu(),
                    self.update_tray_icon(),
                    self.update_tray_tooltip(),
                ])
            }

            Message::UpdateTrayIcon => self.update_tray_icon(),

            Message::UpdateTrayTooltip => self.update_tray_tooltip(),

            Message::UpdateTrayMenu => self.update_tray_menu(),

            Message::Quit => {
                println!("Quitting application...");
                iced::exit()
            }
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            self.plugins.subscriptions().map(From::from),
            self.tray_handle.listen().map(Message::TrayOutput),
        ])
    }

    fn view(&self) -> Element<'_, Message> {
        let status_text = if self.visible {
            "Tray icon is visible"
        } else {
            "Tray icon is hidden (check system tray)"
        };

        let last_action = self
            .last_menu_item
            .as_ref()
            .map(|item| format!("Last menu action: {}", item))
            .unwrap_or_else(|| "No menu actions yet".to_string());

        let content = column![
            text("Tray Icon Plugin Demo").size(32),
            text("").size(10),
            // Status section
            text("Current Status:").size(18),
            row![
                button("🟢 Online").on_press(Message::ChangeStatus(Status::Online)),
                button("🟡 Away").on_press(Message::ChangeStatus(Status::Away)),
                button("🔴 Busy").on_press(Message::ChangeStatus(Status::Busy)),
                button("⚫ Offline").on_press(Message::ChangeStatus(Status::Offline)),
            ]
            .spacing(10),
            text(format!("Active: {}", self.status.as_str())).size(14),
            text("").size(10),
            // Settings section
            text("Settings:").size(18),
            checkbox("Start on Login", self.auto_start).on_toggle(|_| Message::ToggleAutoStart),
            checkbox("Enable Notifications", self.notifications_enabled)
                .on_toggle(|_| Message::ToggleNotifications),
            text("").size(10),
            // Stats section
            text("Statistics:").size(18),
            text(format!("Icon clicks: {}", self.click_count)).size(14),
            text(status_text).size(14),
            text(last_action).size(12),
            text("").size(10),
            // Manual update buttons
            text("Manual Updates:").size(18),
            row![
                button("Update Icon").on_press(Message::UpdateTrayIcon),
                button("Update Tooltip").on_press(Message::UpdateTrayTooltip),
                button("Update Menu").on_press(Message::UpdateTrayMenu),
            ]
            .spacing(10),
            text("").size(10),
            // Info section
            text("Features Demonstrated:").size(18),
            text("✓ Native tray-icon menu API (no wrappers!)").size(12),
            text("✓ Dynamic icon updates (color changes with status)").size(12),
            text("✓ Dynamic tooltip updates").size(12),
            text("✓ Checkable menu items").size(12),
            text("✓ Submenus").size(12),
            text("✓ Menu click event handling").size(12),
            text("✓ Icon click/double-click events").size(12),
            text("").size(10),
            text("Note: Menu state is set at initialization.").size(11),
            text("For dynamic menus, store menu item references").size(11),
            text("and update them directly (see README).").size(11),
            text("").size(10),
            // Actions
            row![
                button("Toggle Tray Icon Visibility").on_press(Message::ToggleVisibility),
                button("Quit").on_press(Message::Quit),
            ]
            .spacing(10),
        ]
        .spacing(10)
        .padding(20);

        scrollable(content)
            .width(iced::Length::Fill)
            .height(iced::Length::Fill)
            .into()
    }
}

/// Create an icon with the specified color
fn create_icon(rgb: [u8; 3]) -> Vec<u8> {
    use std::io::Cursor;

    // Create a 32x32 colored circle
    let width = 32u32;
    let height = 32u32;
    let mut img = image::ImageBuffer::new(width, height);

    for (x, y, pixel) in img.enumerate_pixels_mut() {
        // Create a circle with the specified color
        let center_x = width as f32 / 2.0;
        let center_y = height as f32 / 2.0;
        let radius = (width as f32 / 2.0) - 2.0;

        let dx = x as f32 - center_x;
        let dy = y as f32 - center_y;
        let distance = (dx * dx + dy * dy).sqrt();

        if distance <= radius {
            *pixel = image::Rgba([rgb[0], rgb[1], rgb[2], 255u8]);
        } else if distance <= radius + 2.0 {
            // Anti-aliasing edge
            let alpha = ((radius + 2.0 - distance) / 2.0 * 255.0) as u8;
            *pixel = image::Rgba([rgb[0], rgb[1], rgb[2], alpha]);
        } else {
            *pixel = image::Rgba([0u8, 0u8, 0u8, 0u8]); // Transparent
        }
    }

    let mut bytes: Vec<u8> = Vec::new();
    img.write_to(&mut Cursor::new(&mut bytes), image::ImageFormat::Png)
        .expect("Failed to encode icon");

    bytes
}
