use iced::widget::{button, column, container, text};
use iced::window::Position;
use iced::{Element, Subscription, Task, window};
use iced_plugins::{PluginHandle, PluginManager, PluginMessage};
use iced_window_state_plugin::WindowStatePlugin;
const APP_NAME: &str = "window_state_plugin";

fn main() -> iced::Result {
    let window_state = WindowStatePlugin::load(APP_NAME).unwrap_or_default();

    println!("Loading window state: {:?}", window_state);
    iced::application(App::new, App::update, App::view)
        .subscription(App::subscription)
        .window(window::Settings {
            size: window_state.size,
            position: Position::Specific(window_state.position),
            ..Default::default()
        })
        .run()
}

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
    window_handle: PluginHandle<WindowStatePlugin>,
}

impl App {
    fn new() -> (Self, Task<Message>) {
        let mut plugins = PluginManager::new();

        let window_handle = plugins.install(WindowStatePlugin::new(APP_NAME.to_string()));

        (
            App {
                plugins,
                window_handle,
            },
            Task::none(),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Plugin(plugin_msg) => self.plugins.update(plugin_msg).map(From::from),

            Message::ManualSave => self
                .window_handle
                .dispatch(WindowStatePlugin::force_save())
                .map(From::from),
            Message::ResetWindow => self
                .window_handle
                .dispatch(WindowStatePlugin::reset_to_default())
                .map(From::from),
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        self.plugins.subscriptions().map(From::from)
    }

    fn view(&self) -> Element<'_, Message> {
        let Some((window_state, config_path)) = self
            .plugins
            .get_plugin_state::<WindowStatePlugin>()
            .map(|s| (s.current_state(), s.config_path()))
        else {
            return container(text("No window state found")).into();
        };

        let info_text = format!(
            "Window State:\n\
             Size: {:.0}x{:.0}\n\
             Position: ({:.0}, {:.0})\n\
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
        );

        let path_text = format!("Config: {}", config_path.display());

        let content = column![
            text("Window State Plugin").size(32),
            text(info_text).size(14),
            text(path_text).size(11),
            button("Manual Save")
                .padding([2, 8])
                .on_press(Message::ManualSave),
            button("Reset to Default")
                .padding([2, 8])
                .on_press(Message::ResetWindow),
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
