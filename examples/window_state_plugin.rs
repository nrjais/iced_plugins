use iced::widget::{button, column, container, text};
use iced::window::Position;
use iced::{Element, Subscription, Task, window};
use iced_plugins::{PluginHandle, PluginManager, PluginManagerBuilder, PluginMessage};
use iced_window_state_plugin::{WindowStateMessage, WindowStateOutput, WindowStatePlugin};
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
    PluginOutput(WindowStateOutput),
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
    count: u32,
}

impl App {
    fn new() -> (Self, Task<Message>) {
        // Use the builder pattern to set up plugins
        let (plugins, init_task) = PluginManagerBuilder::new()
            .with_plugin(WindowStatePlugin::new(APP_NAME.to_string()))
            .build();

        // Retrieve handle after building
        let window_handle = plugins.get_handle::<WindowStatePlugin>().unwrap();

        (
            App {
                plugins,
                window_handle,
                count: 0,
            },
            init_task.map(From::from),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Plugin(plugin_msg) => self.plugins.update(plugin_msg).map(From::from),

            Message::ManualSave => self
                .window_handle
                .dispatch(WindowStateMessage::ForceSave)
                .map(From::from),
            Message::ResetWindow => self
                .window_handle
                .dispatch(WindowStateMessage::ResetToDefault)
                .map(From::from),
            Message::PluginOutput(output) => {
                self.count += 1;
                println!("count: {}, output: {:?}", self.count, output);
                Task::none()
            }
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        let window_sub = if self.count < 100 {
            self.window_handle
                .listen_with(|output| matches!(output, WindowStateOutput::StateSaved(_)))
                .map(Message::PluginOutput)
        } else {
            Subscription::none()
        };
        Subscription::batch([self.plugins.subscriptions().map(From::from), window_sub])
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
