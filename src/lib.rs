use iced::Subscription;
use std::any::{Any, TypeId};
use std::sync::Arc;

/// Core trait that all plugins must implement.
/// Plugins can have their own state and respond to messages.
pub trait Plugin: Send + Sync {
    /// The message type this plugin handles
    type Message: Clone + Send + Sync + 'static;

    /// The state type for this plugin
    type State: Send + 'static;

    /// Returns the unique name/identifier for this plugin
    fn name(&self) -> &'static str;

    /// Initialize the plugin and return its initial state
    fn init(&self) -> Self::State;

    /// Update the plugin state based on a message
    /// Returns a Task that can produce more messages
    fn update(&self, state: &mut Self::State, message: Self::Message) -> iced::Task<Self::Message>;

    /// Subscribe to external events
    /// The state is passed as a reference to allow subscription to depend on state
    fn subscription(&self, state: &Self::State) -> Subscription<Self::Message>;
}

/// A type-erased plugin message that can be routed automatically
#[derive(Clone)]
pub struct PluginMessage {
    plugin_index: usize,
    message: Arc<dyn Any + Send + Sync>,
    type_id: TypeId,
}

impl PluginMessage {
    /// Create a new plugin message (internal use)
    fn new<M: 'static + Send + Sync>(plugin_index: usize, message: M) -> Self {
        Self {
            plugin_index,
            type_id: TypeId::of::<M>(),
            message: Arc::new(message),
        }
    }

    /// Get the plugin index this message is for
    pub fn plugin_index(&self) -> usize {
        self.plugin_index
    }
}

/// Holds a single plugin instance with its state and behavior
struct PluginEntry {
    name: &'static str,
    state: Box<dyn Any + Send>,
    message_type_id: TypeId,
    update_fn: Box<
        dyn Fn(&mut dyn Any, Arc<dyn Any + Send + Sync>) -> iced::Task<PluginMessage> + Send + Sync,
    >,
    subscription_fn: Box<dyn Fn(&dyn Any) -> Subscription<PluginMessage> + Send + Sync>,
}

/// Main plugin manager that holds all installed plugins and their states.
/// This struct should be embedded in your application state.
///
/// # Example
/// ```ignore
/// struct App {
///     plugins: PluginManager<Message>,
///     // ... other fields
/// }
/// ```
pub struct PluginManager {
    plugins: Vec<PluginEntry>,
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginManager {
    /// Create a new empty plugin manager
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
        }
    }
}

impl PluginManager {
    /// Install a plugin into the manager.
    /// Returns a message constructor that wraps plugin messages for automatic routing.
    /// Plugins are driven in the order they are installed.
    ///
    /// # Example
    /// ```ignore
    /// let mut manager = PluginManager::new();
    /// let to_plugin_msg = manager.install(MyPlugin);
    /// // Now use to_plugin_msg to wrap plugin messages
    /// ```
    pub fn install<P>(
        &mut self,
        plugin: P,
    ) -> impl Fn(P::Message) -> PluginMessage + Clone + Send + 'static
    where
        P: Plugin + 'static,
    {
        let name = plugin.name();
        let plugin = Arc::new(plugin);
        let state = plugin.init();
        let plugin_index = self.plugins.len();
        let message_type_id = TypeId::of::<P::Message>();

        let plugin_for_update = Arc::clone(&plugin);
        let update_fn = Box::new(
            move |state: &mut dyn Any, message: Arc<dyn Any + Send + Sync>| {
                if let Some(msg) = message.downcast_ref::<P::Message>() {
                    let typed_state = state.downcast_mut::<P::State>().unwrap();
                    let task = plugin_for_update.update(typed_state, msg.clone());
                    // Map plugin message back to AppMsg via PluginMessage
                    task.map(move |plugin_msg| PluginMessage::new(plugin_index, plugin_msg))
                } else {
                    iced::Task::none()
                }
            },
        );

        let subscription_fn = Box::new(move |state: &dyn Any| {
            let typed_state = state.downcast_ref::<P::State>().unwrap();
            plugin
                .subscription(typed_state)
                .map(move |plugin_msg| PluginMessage::new(plugin_index, plugin_msg))
        });

        let entry = PluginEntry {
            name,
            state: Box::new(state),
            message_type_id,
            update_fn,
            subscription_fn,
        };

        self.plugins.push(entry);

        // Return a closure that wraps plugin messages
        move |msg: P::Message| PluginMessage::new(plugin_index, msg)
    }

    /// Update the plugin manager with a plugin message.
    /// This automatically routes the message to the correct plugin.
    ///
    /// # Example
    /// ```ignore
    /// match message {
    ///     Message::Plugin(plugin_msg) => {
    ///         return self.plugins.update(plugin_msg);
    ///     }
    ///     // ... other messages
    /// }
    /// ```
    pub fn update(&mut self, message: PluginMessage) -> iced::Task<PluginMessage> {
        let plugin_index = message.plugin_index;

        if let Some(entry) = self.plugins.get_mut(plugin_index) {
            // Verify the message type matches the plugin
            if entry.message_type_id == message.type_id {
                (entry.update_fn)(entry.state.as_mut(), Arc::clone(&message.message))
            } else {
                iced::Task::none()
            }
        } else {
            iced::Task::none()
        }
    }

    /// Collect all subscriptions from installed plugins
    /// Call this from your application's subscription method
    ///
    /// # Example
    /// ```ignore
    /// fn subscription(&self) -> Subscription<Message> {
    ///     self.plugins.subscriptions()
    /// }
    /// ```
    pub fn subscriptions(&self) -> Subscription<PluginMessage> {
        let subs: Vec<Subscription<PluginMessage>> = self
            .plugins
            .iter()
            .map(|entry| (entry.subscription_fn)(entry.state.as_ref()))
            .collect();

        Subscription::batch(subs)
    }
}

// Methods available for all PluginManager instances
impl PluginManager {
    /// Get the number of installed plugins
    pub fn plugin_count(&self) -> usize {
        self.plugins.len()
    }

    /// Get a list of all installed plugin names in order
    pub fn plugin_names(&self) -> Vec<&'static str> {
        self.plugins.iter().map(|p| p.name).collect()
    }

    /// Get a reference to a plugin's state by name
    /// Use `downcast_ref` on the result to get the concrete type
    ///
    /// # Example
    /// ```ignore
    /// if let Some(state) = manager.get_plugin_state("my_plugin") {
    ///     if let Some(typed) = state.downcast_ref::<MyPluginState>() {
    ///         // Use typed state
    ///     }
    /// }
    /// ```
    pub fn get_plugin_state(&self, name: &str) -> Option<&(dyn Any + Send)> {
        self.plugins
            .iter()
            .find(|p| p.name == name)
            .map(|p| p.state.as_ref())
    }

    /// Get a mutable reference to a plugin's state by name
    /// Use `downcast_mut` on the result to get the concrete type
    pub fn get_plugin_state_mut(&mut self, name: &str) -> Option<&mut (dyn Any + Send)> {
        self.plugins
            .iter_mut()
            .find(|p| p.name == name)
            .map(|p| p.state.as_mut())
    }

    /// Get plugin state with type safety
    /// Returns None if plugin not found or type mismatch
    pub fn get_typed_state<S: 'static>(&self, name: &str) -> Option<&S> {
        self.get_plugin_state(name)
            .and_then(|state| state.downcast_ref::<S>())
    }

    /// Get mutable plugin state with type safety
    /// Returns None if plugin not found or type mismatch
    pub fn get_typed_state_mut<S: 'static>(&mut self, name: &str) -> Option<&mut S> {
        self.get_plugin_state_mut(name)
            .and_then(|state| state.downcast_mut::<S>())
    }
}

/// Builder pattern for constructing a PluginManager
pub struct PluginManagerBuilder {
    plugins: Vec<Box<dyn FnOnce(&mut PluginManager) + Send>>,
}

impl PluginManagerBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
        }
    }

    /// Add a plugin
    pub fn with_plugin<P>(mut self, plugin: P) -> Self
    where
        P: Plugin + 'static,
    {
        self.plugins
            .push(Box::new(move |manager: &mut PluginManager| {
                let _ = manager.install(plugin);
            }));
        self
    }

    /// Build the plugin manager
    pub fn build(self) -> PluginManager {
        let mut manager: PluginManager = PluginManager::new();
        for install_fn in self.plugins {
            install_fn(&mut manager);
        }
        manager
    }
}

impl Default for PluginManagerBuilder {
    fn default() -> Self {
        Self::new()
    }
}
