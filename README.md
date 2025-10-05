# Iced Plugins

A type-safe plugin system for [Iced](https://github.com/iced-rs/iced) applications.

## Features

- **Type-Safe**: Full compile-time type safety with automatic message routing
- **Zero Boilerplate**: Plugins integrate seamlessly with `PluginMessage`
- **State Management**: Each plugin manages its own state
- **Task Support**: Plugins can produce tasks that map back to app messages
- **Subscriptions**: Plugins can subscribe to external events

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

    fn name(&self) -> &'static str {
        "my_plugin"
    }

    fn init(&self) -> Self::State {
        MyState { counter: 0 }
    }

    fn update(&self, state: &mut Self::State, message: Self::Message) -> Task<Self::Message> {
        match message {
            MyMessage::DoSomething => {
                state.counter += 1;
                Task::none()
            }
        }
    }

    fn subscription(&self, _state: &Self::State) -> Subscription<Self::Message> {
        Subscription::none()
    }
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

Licensed under Apache 2.0 or MIT at your option.
