use iced::widget::{button, checkbox, column, row, scrollable, text};
use iced::{Element, Subscription, Task, window};
use iced_plugins::{PluginHandle, PluginManager, PluginManagerBuilder, PluginMessage};
use iced_tray_icon_plugin::{Menu, MenuItem, TrayIconInput, TrayIconOutput, TrayIconPlugin};

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

        // Setup plugins with initial menu
        let initial_menu = Self::build_menu(false, true, Status::Online);

        let mut builder = PluginManagerBuilder::new();
        let tray_handle = builder.install(
            TrayIconPlugin::new("Tray Icon Demo")
                .with_icon(icon_data)
                .with_menu(initial_menu),
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

    fn build_menu(auto_start: bool, notifications: bool, status: Status) -> Menu {
        let mut menu = Menu::new();

        // Show/Hide items
        menu.add_item(MenuItem::new("show", "Show Tray Icon", true));
        menu.add_item(MenuItem::new("hide", "Hide Tray Icon", true));
        menu.add_item(MenuItem::separator());

        // Status submenu
        let status_menu = MenuItem::new_submenu(
            "status_submenu",
            "Status",
            true,
            vec![
                MenuItem::new_check("status_online", "🟢 Online", true, status == Status::Online),
                MenuItem::new_check("status_away", "🟡 Away", true, status == Status::Away),
                MenuItem::new_check("status_busy", "🔴 Busy", true, status == Status::Busy),
                MenuItem::new_check(
                    "status_offline",
                    "⚫ Offline",
                    true,
                    status == Status::Offline,
                ),
            ],
        );
        menu.add_item(status_menu);
        menu.add_item(MenuItem::separator());

        // Settings submenu
        let settings_menu = MenuItem::new_submenu(
            "settings_submenu",
            "Settings",
            true,
            vec![
                MenuItem::new_check("auto_start", "Start on Login", true, auto_start),
                MenuItem::new_check("notifications", "Enable Notifications", true, notifications),
                MenuItem::separator(),
                MenuItem::new("preferences", "Preferences...", true),
            ],
        );
        menu.add_item(settings_menu);
        menu.add_item(MenuItem::separator());

        // About and Quit
        menu.add_item(MenuItem::new("about", "About", true));
        menu.add_item(MenuItem::separator());
        menu.add_item(MenuItem::new("quit", "Quit", true));

        menu
    }

    fn update_tray_menu(&self) -> Task<Message> {
        // Rebuild the menu with current state and send update
        let menu = Self::build_menu(self.auto_start, self.notifications_enabled, self.status);

        println!(
            "Updating menu (auto_start: {}, notifications: {}, status: {})",
            self.auto_start,
            self.notifications_enabled,
            self.status.as_str()
        );

        self.tray_handle
            .dispatch(TrayIconInput::UpdateMenu(menu))
            .map(From::from)
    }

    fn update_tray_icon(&self) -> Task<Message> {
        let icon_data = create_icon(self.status.color());
        self.tray_handle
            .dispatch(TrayIconInput::SetIcon(icon_data))
            .map(From::from)
    }

    fn update_tray_tooltip(&self) -> Task<Message> {
        let tooltip = format!(
            "Tray Demo - {} - Clicks: {}",
            self.status.as_str(),
            self.click_count
        );
        self.tray_handle
            .dispatch(TrayIconInput::SetTooltip(Some(tooltip)))
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
                        .dispatch(TrayIconInput::Show)
                        .map(From::from)
                } else {
                    self.tray_handle
                        .dispatch(TrayIconInput::Hide)
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
            checkbox(self.auto_start)
                .label("Start on Login")
                .on_toggle(|_| Message::ToggleAutoStart),
            checkbox(self.notifications_enabled)
                .label("Enable Notifications")
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
            text("").size(10),
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
