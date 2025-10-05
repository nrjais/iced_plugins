use iced::widget::{button, column, container, text};
use iced::window::Position;
use iced::{Element, Subscription, Task, window};
use iced_plugins::{PluginManager, PluginMessage};
use iced_window_state_plugin::{WindowPluginState, WindowState, WindowStatePlugin};

fn main() -> iced::Result {
    let window_state = WindowStatePlugin::load_from_disk();

    println!("Loading window state: {:?}", window_state);
    iced::application(App::new, App::update, App::view)
        .subscription(App::subscription)
        .window(window::Settings {
            size: window_state.size,
            position: Position::Specific(window_state.position),
            maximized: window_state.maximized,
            ..Default::default()
        })
        .run()
}

// Main Application - Only one message variant for all plugins!
#[derive(Clone)]
enum Message {
    Plugin(PluginMessage),
    ManualSave,
    ResetWindow,
}

impl From<PluginMessage> for Message {
    fn from(msg: PluginMessage) -> Self {
        Message::Plugin(msg)
    }
}

struct App {
    plugins: PluginManager,
}

impl App {
    fn new() -> (Self, Task<Message>) {
        let mut plugins = PluginManager::new();

        // Install the window state plugin - automatic message routing
        let _ = plugins.install(WindowStatePlugin::new());

        (App { plugins }, Task::none())
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Plugin(plugin_msg) => self.plugins.update(plugin_msg).map(From::from),

            Message::ManualSave => {
                // Manually trigger a save
                if let Some(state) = self
                    .plugins
                    .get_typed_state_mut::<WindowPluginState>("window_state")
                {
                    if let Err(e) = state.force_save() {
                        eprintln!("Failed to save: {}", e);
                    } else {
                        println!("Manually saved window state!");
                    }
                }
                Task::none()
            }
            Message::ResetWindow => {
                // Reset to default window state
                if let Some(state) = self
                    .plugins
                    .get_typed_state_mut::<WindowPluginState>("window_state")
                {
                    state.state = WindowState::default();
                    state.mark_dirty();
                    let _ = state.force_save();
                }
                // Note: Window resize/move commands may require specific window ID in iced master
                Task::none()
            }
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        self.plugins.subscriptions().map(From::from)
    }

    fn view(&self) -> Element<'_, Message> {
        let window_state = self
            .plugins
            .get_typed_state::<WindowPluginState>("window_state")
            .map(|s| s.current_state().clone())
            .unwrap_or_default();

        let info_text = format!(
            "Window State:\n\
             Size: {:.0}x{:.0}\n\
             Position: ({:.0}, {:.0})\n\
             Maximized: {}\n\
             \n\
             Move or resize the window.\n\
             The state is automatically saved every 2 seconds.\n\
             \n\
             Try:\n\
             1. Resize this window\n\
             2. Close the app\n\
             3. Run it again - it should restore your size!",
            window_state.size.width,
            window_state.size.height,
            window_state.position.x,
            window_state.position.y,
            window_state.maximized
        );

        let config_path = WindowState::config_path();
        let path_text = format!("Config: {}", config_path.display());

        let content = column![
            text("Window State Plugin").size(32),
            text(info_text).size(14),
            text(path_text).size(11),
            button("Manual Save").on_press(Message::ManualSave),
            button("Reset to Default").on_press(Message::ResetWindow),
        ]
        .spacing(20)
        .padding(20);

        container(content)
            .width(iced::Length::Fill)
            .height(iced::Length::Fill)
            .center_x(300)
            .center_y(250)
            .into()
    }
}
