use iced::widget::{button, column, scrollable, text};
use iced::{Element, Subscription, Task};
use iced_plugins::{Plugin, PluginHandle, PluginManager, PluginManagerBuilder, PluginMessage};
use std::time::Duration;

fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .subscription(App::subscription)
        .run()
}

#[derive(Clone, Debug)]
pub enum CounterMessage {
    Increment,
    Decrement,
}

#[derive(Debug, Clone)]
pub struct CounterState {
    value: i32,
}

#[derive(Debug, Clone)]
pub struct CounterPlugin;

impl Plugin for CounterPlugin {
    type Message = CounterMessage;
    type State = CounterState;
    type Output = ();

    fn name(&self) -> &'static str {
        "counter"
    }

    fn init(&self) -> (Self::State, iced::Task<Self::Message>) {
        (CounterState { value: 0 }, iced::Task::none())
    }

    fn update(
        &self,
        state: &mut Self::State,
        message: Self::Message,
    ) -> (iced::Task<Self::Message>, Option<Self::Output>) {
        match message {
            CounterMessage::Increment => {
                state.value += 1;
                (iced::Task::none(), None)
            }
            CounterMessage::Decrement => {
                state.value -= 1;
                (iced::Task::none(), None)
            }
        }
    }

    fn subscription(&self, _state: &Self::State) -> Subscription<Self::Message> {
        Subscription::none()
    }
}

// Plugin 2: Timer Plugin that auto-increments
#[derive(Clone, Debug)]
pub enum TimerMessage {
    Tick,
}

#[derive(Debug, Clone)]
pub struct TimerState {
    ticks: u32,
}

#[derive(Debug, Clone)]
pub struct TimerPlugin;

impl Plugin for TimerPlugin {
    type Message = TimerMessage;
    type State = TimerState;
    type Output = ();

    fn name(&self) -> &'static str {
        "timer"
    }

    fn init(&self) -> (Self::State, iced::Task<Self::Message>) {
        (TimerState { ticks: 0 }, iced::Task::none())
    }

    fn update(
        &self,
        state: &mut Self::State,
        message: Self::Message,
    ) -> (iced::Task<Self::Message>, Option<Self::Output>) {
        match message {
            TimerMessage::Tick => {
                state.ticks += 1;
                (iced::Task::none(), None)
            }
        }
    }

    fn subscription(&self, _state: &Self::State) -> Subscription<Self::Message> {
        iced::time::every(Duration::from_secs(1)).map(|_| TimerMessage::Tick)
    }
}

#[derive(Clone)]
enum Message {
    Plugin(PluginMessage),
}

impl From<PluginMessage> for Message {
    fn from(msg: PluginMessage) -> Self {
        Message::Plugin(msg)
    }
}

struct App {
    plugins: PluginManager,
    counter_handle: PluginHandle<CounterPlugin>,
}

impl App {
    fn new() -> (Self, Task<Message>) {
        // Use the builder pattern to set up plugins
        let mut builder = PluginManagerBuilder::new()
            .with_plugin(CounterPlugin)
            .with_plugin(TimerPlugin);

        // Retrieve handles after building
        let counter_handle = builder.install(CounterPlugin);
        let (plugins, init_task) = builder.build();

        (
            App {
                plugins,
                counter_handle,
            },
            init_task.map(From::from),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Plugin(plugin_msg) => self.plugins.update(plugin_msg).map(From::from),
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        self.plugins.subscriptions().map(From::from)
    }

    fn view(&self) -> Element<'_, Message> {
        let counter_value = self
            .plugins
            .get_plugin_state::<CounterPlugin>()
            .map(|s| s.value)
            .unwrap_or(0);

        let timer_ticks = self
            .plugins
            .get_plugin_state::<TimerPlugin>()
            .map(|s| s.ticks)
            .unwrap_or(0);

        let content = column![
            text("Iced Plugin System - Type-Safe!").size(40),
            text(format!("Counter: {}", counter_value)).size(30),
            button("Increment").on_press(Message::Plugin(
                self.counter_handle.message(CounterMessage::Increment)
            )),
            button("Decrement").on_press(Message::Plugin(
                self.counter_handle.message(CounterMessage::Decrement)
            )),
            text(format!("Timer Ticks: {}", timer_ticks)).size(30),
            text(format!(
                "Installed Plugins: {:?}",
                self.plugins.plugin_names()
            )),
        ]
        .spacing(20)
        .padding(20);

        scrollable(content)
            .width(iced::Length::Fill)
            .height(iced::Length::Fill)
            .into()
    }
}
