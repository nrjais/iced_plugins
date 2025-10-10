# Iced Plugins

A type-safe plugin system for [Iced](https://github.com/iced-rs/iced) applications.

## Features

- **Type-Safe**: Full compile-time type safety with automatic message routing
- **Zero Boilerplate**: Plugins integrate seamlessly with `PluginMessage`
- **State Management**: Each plugin manages its own state
- **Task Support**: Plugins can produce background tasks
- **Subscriptions**: Plugins can subscribe to external events
- **Output Streams**: Subscribe to plugin output messages, with filtering

## Quick Start

```rust
use iced::{Element, Subscription, Task};
use iced_plugins::{Plugin, PluginManager, PluginManagerBuilder, PluginMessage};

// 1. Define your app with PluginManager
struct App {
    plugins: PluginManager,
}

#[derive(Clone)]
enum Message {
    Plugin(PluginMessage),
    // ... your messages
}

impl From<PluginMessage> for Message {
    fn from(msg: PluginMessage) -> Self {
        Message::Plugin(msg)
    }
}

// 2. Install plugins during initialization using the builder
fn new() -> (App, Task<Message>) {
    let (plugins, init_task) = PluginManagerBuilder::new()
        .with_plugin(MyPlugin)
        .build();

    (App { plugins }, init_task.map(From::from))
}

// 3. Route plugin messages in update
fn update(&mut self, message: Message) -> Task<Message> {
    match message {
        Message::Plugin(msg) => self.plugins.update(msg).map(From::from),
        // ... other messages
    }
}

// 4. Include plugin subscriptions
fn subscription(&self) -> Subscription<Message> {
    self.plugins.subscriptions().map(From::from)
}
```

## Creating a Plugin

```rust
use iced::{Subscription, Task};
use iced_plugins::Plugin;

pub struct MyPlugin;

// Public Input API - this is what applications dispatch
#[derive(Clone, Debug)]
pub enum MyInput {
    DoSomething,
}

// Internal message type - can be the same as Input for simple plugins
#[derive(Clone, Debug)]
pub enum MyMessage {
    DoSomething,
}

impl From<MyInput> for MyMessage {
    fn from(input: MyInput) -> Self {
        match input {
            MyInput::DoSomething => MyMessage::DoSomething,
        }
    }
}

#[derive(Debug)]
pub struct MyState {
    counter: u32,
}

impl Plugin for MyPlugin {
    type Input = MyInput;
    type Message = MyMessage;
    type State = MyState;
    type Output = ();  // Or your output message type

    fn name(&self) -> &'static str {
        "my_plugin"
    }

    fn init(&self) -> (Self::State, Task<Self::Message>) {
        (MyState { counter: 0 }, Task::none())
    }

    fn update(&self, state: &mut Self::State, message: Self::Message) -> (Task<Self::Message>, Option<Self::Output>) {
        match message {
            MyMessage::DoSomething => {
                state.counter += 1;
                (Task::none(), None)
            }
        }
    }

    fn subscription(&self, _state: &Self::State) -> Subscription<Self::Message> {
        Subscription::none()
    }
}
```

## Subscribing to Plugin Outputs

Plugins can emit output messages that you can subscribe to:

```rust
#[derive(Clone, Debug)]
pub enum MyOutput {
    CounterChanged(u32),
    TaskCompleted,
}

impl Plugin for MyPlugin {
    type Output = MyOutput;

    fn update(&self, state: &mut Self::State, message: Self::Message) -> (Task<Self::Message>, Option<Self::Output>) {
        match message {
            MyMessage::DoSomething => {
                state.counter += 1;
                // Emit output message
                (Task::none(), Some(MyOutput::CounterChanged(state.counter)))
            }
        }
    }
}

// In your app, subscribe to plugin outputs:
use iced_plugins::{PluginManager, PluginHandle};

enum Message {
    Plugin(PluginMessage),
    PluginOutput(MyOutput),
}

struct App {
    plugins: PluginManager,
    my_plugin_handle: PluginHandle<MyPlugin>,
}

fn subscription(&self) -> Subscription<Message> {
    Subscription::batch([
        self.plugins.subscriptions().map(Message::Plugin),
        // Listen to all outputs
        self.my_plugin_handle.listen().map(Message::PluginOutput),
    ])
}
```

### Filtering Plugin Outputs

You can filter and transform outputs to only receive specific events:

```rust
fn subscription(&self) -> Subscription<Message> {
    Subscription::batch([
        self.plugins.subscriptions().map(Message::Plugin),
        // Only receive CounterChanged outputs
        self.my_plugin_handle
            .listen_with(|output| {
                matches!(output, MyOutput::CounterChanged(_)).then_some(Message::PluginOutput(output))
            }),
    ])
}
```

## Using Plugin Handles

Plugin handles let you dispatch messages to plugins:

```rust
use iced_plugins::{PluginManagerBuilder, PluginHandle};

// Get handle when installing with builder
let mut builder = PluginManagerBuilder::new();
let handle = builder.install(MyPlugin);
let (plugins, init_task) = builder.build();

// Or retrieve handle after building
let (plugins, init_task) = PluginManagerBuilder::new()
    .with_plugin(MyPlugin)
    .build();
let handle = plugins.get_handle::<MyPlugin>().unwrap();

// Dispatch messages from anywhere in your app
Message::ButtonClick => {
    handle.dispatch(MyInput::DoSomething).map(From::from)
}

// Or create plugin messages directly for immediate use
use iced::widget::button;

button("Do Something").on_press(Message::Plugin(
    handle.input(MyInput::DoSomething)
))
```

## Available Plugins

- **[window_state](plugins/window_state)** - Automatically save and restore window size/position
- **[auto_updater](plugins/auto_updater)** - Automatic updates from GitHub releases with SHA256 verification (macOS)
- **[store](plugins/store)** - Simple JSON-based store with automatic persistence
- **[tray_icon](plugins/tray_icon)** - System tray icon with menu support (Windows, macOS, Linux)

## Examples

- `cargo run --example counter_plugins` - Multiple plugins working together
- `cargo run --example window_state_plugin` - Window state persistence
- `cargo run --example auto_updater_plugin` - Automatic updates from GitHub
- `cargo run --example store_plugin` - Simple JSON-based data storage
- `cargo run --example tray_icon_plugin` - System tray icon with menu
