use iced::widget::{button, column, container, text};
use iced::{Element, Subscription, Task};
use iced_plugins::{Plugin, PluginHandle, PluginManager, PluginMessage};
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

pub struct CounterState {
    value: i32,
}

pub struct CounterPlugin;

impl Plugin for CounterPlugin {
    type Message = CounterMessage;
    type State = CounterState;

    fn name(&self) -> &'static str {
        "counter"
    }

    fn init(&self) -> Self::State {
        CounterState { value: 0 }
    }

    fn update(&self, state: &mut Self::State, message: Self::Message) -> iced::Task<Self::Message> {
        match message {
            CounterMessage::Increment => {
                state.value += 1;
                iced::Task::none()
            }
            CounterMessage::Decrement => {
                state.value -= 1;
                iced::Task::none()
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

pub struct TimerState {
    ticks: u32,
}

pub struct TimerPlugin;

impl Plugin for TimerPlugin {
    type Message = TimerMessage;
    type State = TimerState;

    fn name(&self) -> &'static str {
        "timer"
    }

    fn init(&self) -> Self::State {
        TimerState { ticks: 0 }
    }

    fn update(&self, state: &mut Self::State, message: Self::Message) -> iced::Task<Self::Message> {
        match message {
            TimerMessage::Tick => {
                state.ticks += 1;
                iced::Task::none()
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
        let mut plugins = PluginManager::new();

        let counter_handle = plugins.install(CounterPlugin);
        let _ = plugins.install(TimerPlugin);

        (
            App {
                plugins,
                counter_handle,
            },
            Task::none(),
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
            text("Note: Only one Plugin(PluginMessage) variant in Message enum!").size(12),
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
