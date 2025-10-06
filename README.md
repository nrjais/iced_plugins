# Iced Plugins

A type-safe plugin system for [Iced](https://github.com/iced-rs/iced) applications.

## Features

- **Type-Safe**: Full compile-time type safety with automatic message routing
- **Zero Boilerplate**: Plugins integrate seamlessly with `PluginMessage`
- **State Management**: Each plugin manages its own state
- **Task Support**: Plugins can produce background tasks
- **Subscriptions**: Plugins can subscribe to external events
- **Output Streams**: Subscribe to plugin output messages, withe filtering

## Quick Start

```rust
use iced::{Element, Subscription, Task};
use iced_plugins::{Plugin, PluginManager, PluginMessage};

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

// 2. Install plugins during initialization
fn new() -> (App, Task<Message>) {
    let mut plugins = PluginManager::new();
    let _handle = plugins.install(MyPlugin::new());

    (App { plugins }, Task::none())
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

#[derive(Clone)]
pub enum MyMessage {
    DoSomething,
}

pub struct MyState {
    counter: u32,
}

impl Plugin for MyPlugin {
    type Message = MyMessage;
    type State = MyState;
    type Output = ();  // Or your output message type

    fn name(&self) -> &'static str {
        "my_plugin"
    }

    fn init(&self) -> Self::State {
        MyState { counter: 0 }
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
enum Message {
    Plugin(PluginMessage),
    PluginOutput(MyOutput),
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

You can filter outputs to only receive specific events:

```rust
fn subscription(&self) -> Subscription<Message> {
    Subscription::batch([
        self.plugins.subscriptions().map(Message::Plugin),
        // Only receive CounterChanged outputs
        self.my_plugin_handle
            .listen_filtered(|output| {
                matches!(output, MyOutput::CounterChanged(_))
            })
            .map(Message::PluginOutput),
    ])
}
```

## Using Plugin Handles

Plugin handles let you dispatch messages to plugins:

```rust
// Get handle when installing
let handle = plugins.install(MyPlugin::new());

// Dispatch messages from anywhere in your app
Message::ButtonClick => {
    handle.dispatch(MyMessage::DoSomething).map(From::from)
}
```

## Available Plugins

- **[window_state](plugins/window_state)** - Automatically save and restore window size/position

## Examples

- `cargo run --example counter_plugins` - Multiple plugins working together
- `cargo run --example window_state_plugin` - Window state persistence

## License

