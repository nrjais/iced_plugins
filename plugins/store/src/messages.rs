//! Message types for the store plugin

use serde::{Serialize, de::DeserializeOwned};

/// Public input API that applications use to interact with the store plugin
///
/// This is the primary interface for sending commands to the store plugin.
///
/// # Example
///
/// ```ignore
/// use iced_store_plugin::{StoreInput, AppName};
///
/// let app_name = AppName::new("com", "example", "myapp");
/// let input = StoreInput::set("settings", "theme", "dark");
/// // dispatch to plugin using handle.dispatch(input)
/// ```
#[derive(Clone, Debug)]
pub enum StoreInput {
    /// Set a value in the store
    Set {
        group: String,
        key: String,
        value: String,
    },
    /// Get a value from the store
    Get { group: String, key: String },
    /// Delete a value from the store
    Delete { group: String, key: String },
}

impl From<StoreInput> for StoreMessage {
    fn from(input: StoreInput) -> Self {
        match input {
            StoreInput::Set { group, key, value } => StoreMessage::Set { group, key, value },
            StoreInput::Get { group, key } => StoreMessage::Get { group, key },
            StoreInput::Delete { group, key } => StoreMessage::Delete { group, key },
        }
    }
}

impl StoreInput {
    /// Create a Set input with automatic serialization
    ///
    /// # Example
    ///
    /// ```ignore
    /// use iced_store_plugin::StoreInput;
    /// use serde::Serialize;
    ///
    /// #[derive(Serialize)]
    /// struct Config { theme: String }
    ///
    /// let config = Config { theme: "dark".to_string() };
    /// let input = StoreInput::set("settings", "config", config);
    /// ```
    pub fn set<T>(group: impl Into<String>, key: impl Into<String>, value: T) -> Self
    where
        T: Serialize,
    {
        let value = serde_json::to_string(&value).unwrap_or_else(|e| {
            eprintln!("Failed to serialize value: {}", e);
            String::new()
        });

        Self::Set {
            group: group.into(),
            key: key.into(),
            value,
        }
    }

    /// Create a Get input
    ///
    /// # Example
    ///
    /// ```ignore
    /// use iced_store_plugin::StoreInput;
    ///
    /// let input = StoreInput::get("settings", "theme");
    /// ```
    pub fn get(group: impl Into<String>, key: impl Into<String>) -> Self {
        Self::Get {
            group: group.into(),
            key: key.into(),
        }
    }

    /// Create a Delete input
    ///
    /// # Example
    ///
    /// ```ignore
    /// use iced_store_plugin::StoreInput;
    ///
    /// let input = StoreInput::delete("settings", "theme");
    /// ```
    pub fn delete(group: impl Into<String>, key: impl Into<String>) -> Self {
        Self::Delete {
            group: group.into(),
            key: key.into(),
        }
    }
}

/// Internal messages that the store plugin handles
///
/// Note: This is for internal use. Applications should use `StoreInput` instead.
#[derive(Clone, Debug)]
pub enum StoreMessage {
    /// Set a value
    Set {
        group: String,
        key: String,
        value: String,
    },
    /// Get a value
    Get { group: String, key: String },
    /// Delete a value
    Delete { group: String, key: String },
    /// Save result
    SaveResult { group: String, success: bool },
    /// Get result
    GetResult {
        group: String,
        key: String,
        value: Option<String>,
    },
}

/// Output messages emitted by the store plugin
///
/// These are the responses from the store plugin that applications can handle.
///
/// # Example
///
/// ```ignore
/// use iced_store_plugin::StoreOutput;
///
/// fn handle_output(output: StoreOutput) {
///     match output {
///         StoreOutput::Get { value, .. } => {
///             // Use the value
///         }
///         StoreOutput::NotFound { key, .. } => {
///             println!("Key not found: {}", key);
///         }
///         _ => {}
///     }
/// }
/// ```
#[derive(Clone, Debug)]
pub enum StoreOutput {
    /// A value was set successfully
    Set { group: String, key: String },
    /// A value was retrieved
    Get {
        group: String,
        key: String,
        value: String,
    },
    /// A value was not found
    NotFound { group: String, key: String },
    /// A value was deleted successfully
    Deleted { group: String, key: String },
    /// An error occurred
    Error { message: String },
}

impl StoreOutput {
    /// Try to deserialize a retrieved value
    ///
    /// Returns `Some(T)` if this is a `Get` output and the value can be deserialized,
    /// otherwise returns `None`.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use iced_store_plugin::StoreOutput;
    /// use serde::Deserialize;
    ///
    /// #[derive(Deserialize)]
    /// struct Config { theme: String }
    ///
    /// fn handle_output(output: StoreOutput) {
    ///     if let Some(config) = output.as_value::<Config>() {
    ///         println!("Theme: {}", config.theme);
    ///     }
    /// }
    /// ```
    pub fn as_value<T: DeserializeOwned>(&self) -> Option<T> {
        match self {
            StoreOutput::Get { value, .. } => serde_json::from_str(value).ok(),
            _ => None,
        }
    }
}
