use iced::widget::{button, column, container, text};
use iced::{Element, Subscription, Task};
use iced_plugins::{Plugin, PluginManager, PluginMessage};
use std::time::Duration;

fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .subscription(App::subscription)
        .run()
}

// Plugin 1: Counter Plugin
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

// Main Application - Only one message variant for all plugins!
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
    // Store message constructors for UI interactions
    counter_msg: Box<dyn Fn(CounterMessage) -> Message + Send + Sync>,
}

impl App {
    fn new() -> (Self, Task<Message>) {
        let mut plugins = PluginManager::new();

        // Install plugins and get message constructors
        let counter_msg = plugins.install(CounterPlugin);
        let _ = plugins.install(TimerPlugin);

        (
            App {
                plugins,
                counter_msg: Box::new(move |msg| Message::Plugin(counter_msg(msg))),
            },
            Task::none(),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        // Automatic routing - plugin manager handles everything!
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
            .get_typed_state::<CounterState>("counter")
            .map(|s| s.value)
            .unwrap_or(0);

        let timer_ticks = self
            .plugins
            .get_typed_state::<TimerState>("timer")
            .map(|s| s.ticks)
            .unwrap_or(0);

        let content = column![
            text("Iced Plugin System - Type-Safe!").size(40),
            text(format!("Counter: {}", counter_value)).size(30),
            button("Increment").on_press((self.counter_msg)(CounterMessage::Increment)),
            button("Decrement").on_press((self.counter_msg)(CounterMessage::Decrement)),
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
