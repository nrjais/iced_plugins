use iced::futures::channel::mpsc;
use iced::{Subscription, Task};
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::{Arc, Mutex};

/// Core trait that all plugins must implement.
/// Plugins can have their own state and respond to messages.
pub trait Plugin: Send + Sync + Debug {
    /// The message type this plugin handles
    type Message: Clone + Send + Sync + Debug + 'static;

    /// The state type for this plugin
    type State: Send + Debug + 'static;

    /// The output message type this plugin can emit
    /// These can be subscribed to by application code
    type Output: Clone + Send + Sync + 'static;

    /// Returns the unique name/identifier for this plugin
    fn name(&self) -> &'static str;

    /// Initialize the plugin and return its initial state
    fn init(&self) -> (Self::State, Task<Self::Message>);

    /// Update the plugin state based on a message
    /// Returns a Task that can produce more messages and an optional output message
    fn update(
        &self,
        state: &mut Self::State,
        message: Self::Message,
    ) -> (Task<Self::Message>, Option<Self::Output>);

    /// Subscribe to external events
    /// The state is passed as a reference to allow subscription to depend on state
    fn subscription(&self, state: &Self::State) -> Subscription<Self::Message>;
}

/// Shared registry for managing output subscriptions
type OutputRegistry = Arc<Mutex<HashMap<usize, Vec<mpsc::UnboundedSender<PluginOutput>>>>>;

/// Creates a stream that listens for plugin outputs with optional filtering
fn output_listener_filtered<O: Clone + Send + Sync + 'static, R>(
    plugin_index: usize,
    output_type_id: TypeId,
    registry: OutputRegistry,
    filter: Arc<dyn Fn(O) -> Option<R> + Send + Sync>,
) -> impl iced::futures::Stream<Item = R> {
    use iced::futures::{SinkExt, StreamExt};

    iced::stream::channel(100, move |mut output_sender: mpsc::Sender<R>| async move {
        let (sender, mut receiver) = mpsc::unbounded();

        if let Ok(mut reg) = registry.lock() {
            reg.entry(plugin_index)
                .or_insert_with(Vec::new)
                .push(sender);
        }

        loop {
            match receiver.next().await {
                Some(output) => {
                    if plugin_index == output.plugin_index()
                        && output_type_id == output.type_id
                        && let Some(output) = output.downcast::<O>()
                    {
                        if let Some(event) = filter(output.clone())
                            && output_sender.send(event).await.is_err()
                        {
                            break;
                        };
                    }
                }
                None => break,
            }
        }
    })
}

/// A handle to a plugin that allows creating tasks for it
#[derive(Clone, Debug)]
pub struct PluginHandle<P: Plugin> {
    plugin_index: usize,
    output_registry: OutputRegistry,
    _phantom: std::marker::PhantomData<P>,
}

impl<P: Plugin> PluginHandle<P> {
    fn new(plugin_index: usize, output_registry: OutputRegistry) -> Self {
        Self {
            plugin_index,
            output_registry,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Create a task that dispatches a message to this plugin
    ///
    /// # Example
    /// ```ignore
    /// let handle = plugins.install(MyPlugin);
    /// let task = handle.dispatch(MyMessage::DoSomething);
    /// ```
    pub fn dispatch(&self, message: P::Message) -> Task<PluginMessage> {
        let plugin_msg = PluginMessage::new(self.plugin_index, message);
        Task::done(plugin_msg)
    }

    /// Wrap a plugin message into a PluginMessage
    pub fn message(&self, message: P::Message) -> PluginMessage {
        PluginMessage::new(self.plugin_index, message)
    }

    /// Subscribe to outputs from this plugin with an optional filter
    ///
    /// Creates a subscription that will receive outputs emitted by this plugin.
    /// If a filter is provided, only outputs that pass the filter will be received.
    /// When the subscription ends, it is automatically cleaned up.
    ///
    /// # Example
    /// ```ignore
    /// // Listen to all outputs
    /// fn subscription(&self) -> Subscription<Message> {
    ///     Subscription::batch([
    ///         self.plugins.subscriptions().map(Message::Plugin),
    ///         self.window_handle.listen().map(Message::WindowOutput),
    ///     ])
    /// }
    /// ```
    pub fn listen(&self) -> iced::Subscription<P::Output> {
        self.listen_with_filter(Arc::new(|output| Some(output)))
    }

    /// Subscribe to filtered outputs from this plugin
    ///
    /// Creates a subscription that will only receive outputs that pass the filter predicate.
    ///
    /// # Example
    /// ```ignore
    /// // Only listen to specific window events
    /// fn subscription(&self) -> Subscription<Message> {
    ///     self.window_handle
    ///         .listen_filtered(|output| {
    ///             matches!(output, WindowStateOutput::Saved)
    ///         })
    ///         .map(Message::WindowOutput)
    /// }
    /// ```
    pub fn listen_with<F, O>(&self, filter: F) -> iced::Subscription<O>
    where
        F: Fn(P::Output) -> Option<O> + Send + Sync + 'static,
        O: Clone + Send + Sync + 'static,
    {
        self.listen_with_filter(Arc::new(filter))
    }

    fn listen_with_filter<O: Clone + Send + Sync + 'static>(
        &self,
        filter: Arc<dyn Fn(P::Output) -> Option<O> + Send + Sync + 'static>,
    ) -> iced::Subscription<O> {
        struct ListenState<O, R> {
            plugin_index: usize,
            output_type_id: TypeId,
            registry: OutputRegistry,
            filter: Arc<dyn Fn(O) -> Option<R> + Send + Sync>,
            filter_id: u64,
            _phantom: std::marker::PhantomData<O>,
            _phantom_r: std::marker::PhantomData<R>,
        }

        impl<O, R> std::hash::Hash for ListenState<O, R> {
            fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
                self.plugin_index.hash(state);
                std::any::type_name::<O>().hash(state);
                self.filter_id.hash(state);
            }
        }

        impl<O, R> Clone for ListenState<O, R> {
            fn clone(&self) -> Self {
                Self {
                    plugin_index: self.plugin_index,
                    output_type_id: self.output_type_id,
                    registry: Arc::clone(&self.registry),
                    filter: self.filter.clone(),
                    filter_id: self.filter_id,
                    _phantom: std::marker::PhantomData,
                    _phantom_r: std::marker::PhantomData,
                }
            }
        }

        fn create_stream<O: Clone + Send + Sync + 'static, R: Clone + Send + Sync + 'static>(
            state: &ListenState<O, R>,
        ) -> iced::futures::stream::BoxStream<'static, R> {
            Box::pin(output_listener_filtered::<O, R>(
                state.plugin_index,
                state.output_type_id,
                Arc::clone(&state.registry),
                state.filter.clone(),
            ))
        }

        let state = ListenState::<P::Output, O> {
            plugin_index: self.plugin_index,
            output_type_id: TypeId::of::<P::Output>(),
            registry: Arc::clone(&self.output_registry),
            filter_id: Arc::as_ptr(&filter) as *const () as u64,
            filter,
            _phantom: std::marker::PhantomData,
            _phantom_r: std::marker::PhantomData,
        };

        iced::Subscription::run_with(state, create_stream::<P::Output, O>)
    }
}

/// A type-erased plugin message that can be routed automatically
#[derive(Clone, Debug)]
pub struct PluginMessage {
    plugin_index: usize,
    message: Arc<dyn Any + Send + Sync>,
    type_id: TypeId,
}

impl PluginMessage {
    /// Create a new plugin message
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

/// Non-capturing function pointer for plugin subscriptions
fn plugin_subscription_fn<P: Plugin + 'static>(
    state: &dyn Any,
    plugin: &AnyRef,
    plugin_index: usize,
) -> Subscription<PluginMessage> {
    let typed_state = state.downcast_ref::<P::State>().unwrap();
    let typed_plugin = plugin.downcast_ref::<Arc<P>>().unwrap();
    let inner_sub = typed_plugin.subscription(typed_state);

    inner_sub
        .with(plugin_index)
        .map(|(plugin_index, msg)| PluginMessage::new(plugin_index, msg))
}

/// Type-erased output message from a plugin
#[derive(Clone)]
pub struct PluginOutput {
    plugin_index: usize,
    output: Arc<dyn Any + Send + Sync>,
    type_id: TypeId,
}

impl PluginOutput {
    fn new<O: 'static + Send + Sync>(plugin_index: usize, output: O) -> Self {
        Self {
            plugin_index,
            type_id: TypeId::of::<O>(),
            output: Arc::new(output),
        }
    }

    /// Get the plugin index this output is from
    pub fn plugin_index(&self) -> usize {
        self.plugin_index
    }

    /// Try to downcast the output to a specific type
    pub fn downcast<O: 'static>(&self) -> Option<&O> {
        if self.type_id == TypeId::of::<O>() {
            self.output.downcast_ref::<O>()
        } else {
            None
        }
    }
}

impl std::fmt::Debug for PluginOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "PluginOutput {{ plugin_index: {}, type_id: {:?} }}",
            self.plugin_index, self.type_id
        )
    }
}

type AnyRef = dyn Any + Send + Sync;
type AnyPlugin = Arc<dyn Any + Send + Sync>;
type AnyMessage = Arc<dyn Any + Send + Sync>;

/// Holds a single plugin instance with its state and behavior
struct PluginEntry {
    name: &'static str,
    state: Box<dyn Any + Send>,
    plugin_type: TypeId,
    message_type_id: TypeId,
    output_type_id: TypeId,
    plugin: AnyPlugin,
    plugin_index: usize,
    update_fn: Box<
        dyn Fn(&mut dyn Any, AnyMessage) -> (Task<PluginMessage>, Option<PluginOutput>)
            + Send
            + Sync,
    >,
    subscription_fn: fn(&dyn Any, &AnyRef, usize) -> Subscription<PluginMessage>,
}

impl std::fmt::Debug for PluginEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "PluginEntry {{ name: {}, plugin_type: {:?}, message_type_id: {:?}, output_type_id: {:?}, state: {:?} }}",
            self.name, self.plugin_type, self.message_type_id, self.output_type_id, self.state
        )
    }
}

/// Main plugin manager that holds all installed plugins and their states.
/// This struct should be embedded in your application state.
///
/// # Example
/// ```ignore
/// struct App {
///     plugins: PluginManager,
///     // ... other fields
/// }
/// ```
pub struct PluginManager {
    plugins: Vec<PluginEntry>,
    output_registry: OutputRegistry,
}

impl std::fmt::Debug for PluginManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PluginManager {{ plugins: {:?} }}", self.plugins,)
    }
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
            output_registry: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl PluginManager {
    /// Internal method to install a plugin into the manager.
    /// Returns the initial task for this plugin.
    ///
    /// Users should use PluginManagerBuilder to install plugins instead.
    fn install_internal<P>(&mut self, plugin: P) -> (PluginHandle<P>, Task<PluginMessage>)
    where
        P: Plugin + 'static,
    {
        let name = plugin.name();
        let plugin = Arc::new(plugin);
        let (state, init_task) = plugin.init();
        let plugin_index = self.plugins.len();
        let message_type_id = TypeId::of::<P::Message>();
        let output_type_id = TypeId::of::<P::Output>();

        let plugin_for_update = Arc::clone(&plugin);
        let update_fn = Box::new(move |state: &mut dyn Any, message: AnyMessage| {
            if let Some(msg) = message.downcast_ref::<P::Message>()
                && let Some(typed_state) = state.downcast_mut::<P::State>()
            {
                let (task, output) = plugin_for_update.update(typed_state, msg.clone());
                let task = task.map(move |plugin_msg| PluginMessage::new(plugin_index, plugin_msg));
                let plugin_output = output.map(|o| PluginOutput::new(plugin_index, o));
                (task, plugin_output)
            } else {
                (Task::none(), None)
            }
        });

        let entry = PluginEntry {
            name,
            state: Box::new(state),
            plugin_type: TypeId::of::<P>(),
            message_type_id,
            output_type_id,
            plugin: Arc::new(plugin),
            plugin_index,
            update_fn,
            subscription_fn: plugin_subscription_fn::<P>,
        };

        self.plugins.push(entry);
        let handle = PluginHandle::new(plugin_index, Arc::clone(&self.output_registry));
        (
            handle,
            init_task.map(move |msg| PluginMessage::new(plugin_index, msg)),
        )
    }

    /// Update the plugin manager with a plugin message.
    /// This automatically routes the message to the correct plugin and distributes outputs to subscribers.
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
    pub fn update(&mut self, message: PluginMessage) -> Task<PluginMessage> {
        let plugin_index = message.plugin_index;

        if let Some(entry) = self.plugins.get_mut(plugin_index)
            && entry.message_type_id == message.type_id
        {
            let (task, output) =
                (entry.update_fn)(entry.state.as_mut(), Arc::clone(&message.message));

            if let Some(output) = output
                && let Ok(mut registry) = self.output_registry.lock()
                && let Some(senders) = registry.get_mut(&plugin_index)
            {
                senders.retain(|sender| sender.unbounded_send(output.clone()).is_ok());
            }

            task
        } else {
            Task::none()
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
            .map(|entry| {
                (entry.subscription_fn)(
                    entry.state.as_ref(),
                    entry.plugin.as_ref(),
                    entry.plugin_index,
                )
            })
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

    pub fn get_plugin_state<P: Plugin + 'static>(&self) -> Option<&P::State> {
        self.plugins
            .iter()
            .find(|p| TypeId::of::<P>() == p.plugin_type)
            .map(|p| p.state.as_ref())
            .and_then(|state| state.downcast_ref::<P::State>())
    }

    pub fn get_plugin_state_mut<P: Plugin + 'static>(&mut self) -> Option<&mut P::State> {
        self.plugins
            .iter_mut()
            .find(|p| TypeId::of::<P>() == p.plugin_type)
            .map(|p| p.state.as_mut())
            .and_then(|state| state.downcast_mut::<P::State>())
    }

    /// Get a handle to an installed plugin by its type.
    /// Returns None if the plugin is not installed.
    ///
    /// # Example
    /// ```ignore
    /// let handle: Option<PluginHandle<MyPlugin>> = manager.get_handle();
    /// if let Some(handle) = handle {
    ///     // Use handle to dispatch messages
    ///     let task = handle.dispatch(MyMessage::DoSomething);
    /// }
    /// ```
    pub fn get_handle<P: Plugin + 'static>(&self) -> Option<PluginHandle<P>> {
        self.plugins
            .iter()
            .find(|p| TypeId::of::<P>() == p.plugin_type)
            .map(|p| PluginHandle::new(p.plugin_index, Arc::clone(&self.output_registry)))
    }
}

/// Builder pattern for constructing a PluginManager
///
/// This is the recommended way to set up plugins. It collects all initialization tasks
/// and allows you to retrieve plugin handles after building.
///
/// # Example
/// ```ignore
/// let (plugins, init_task) = PluginManagerBuilder::new()
///     .with_plugin(CounterPlugin)
///     .with_plugin(TimerPlugin)
///     .build();
///
/// // Retrieve handles after building
/// let counter_handle = plugins.get_handle::<CounterPlugin>().unwrap();
/// ```
pub struct PluginManagerBuilder {
    manager: PluginManager,
    tasks: Vec<Task<PluginMessage>>,
}

impl PluginManagerBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            manager: PluginManager::new(),
            tasks: Vec::new(),
        }
    }

    /// Add a plugin to the builder
    pub fn with_plugin<P>(mut self, plugin: P) -> Self
    where
        P: Plugin + 'static,
    {
        let (_, task) = self.manager.install_internal(plugin);
        self.tasks.push(task);
        self
    }

    /// Install a plugin and return a handle to it
    pub fn install<P>(&mut self, plugin: P) -> PluginHandle<P>
    where
        P: Plugin + 'static,
    {
        let (handle, task) = self.manager.install_internal(plugin);
        self.tasks.push(task);
        handle
    }

    /// Build the plugin manager and return it with all batched init tasks
    ///
    /// Returns a tuple of (PluginManager, Task) where the task contains all
    /// plugin initialization tasks batched together. Map this task to your
    /// application's message type.
    ///
    /// After building, use `get_handle()` to retrieve handles to installed plugins.
    pub fn build(self) -> (PluginManager, Task<PluginMessage>) {
        let combined_task = Task::batch(self.tasks);
        (self.manager, combined_task)
    }
}

impl Default for PluginManagerBuilder {
    fn default() -> Self {
        Self::new()
    }
}
