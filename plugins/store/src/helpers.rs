//! Standalone helper functions for direct store access
//!
//! These functions allow you to read and write to the store without
//! initializing the plugin system. Useful for CLI tools, scripts, or
//! accessing data outside of the main application.

use crate::app_name::AppName;
use crate::storage::{load_group, modify_group};
use serde::{Serialize, de::DeserializeOwned};

/// Read a value from the store
///
/// # Arguments
///
/// * `app_name` - The application name
/// * `group` - The group name (e.g., "settings", "cache")
/// * `key` - The key to read
///
/// # Returns
///
/// Returns the deserialized value if found and valid.
///
/// # Errors
///
/// Returns an error if the group cannot be loaded, the key is not found,
/// or the value cannot be deserialized.
///
/// # Example
///
/// ```ignore
/// use iced_store_plugin::{AppName, read_value};
/// use serde::Deserialize;
///
/// #[derive(Deserialize)]
/// struct UserPrefs {
///     theme: String,
///     font_size: u32,
/// }
///
/// async fn load_prefs() -> Result<UserPrefs, String> {
///     let app_name = AppName::new("com", "example", "myapp");
///     read_value(&app_name, "settings", "user_prefs").await
/// }
/// ```
pub async fn read_value<T>(app_name: &AppName, group: &str, key: &str) -> Result<T, String>
where
    T: DeserializeOwned,
{
    let data = load_group(app_name, group).await?;

    let value = data
        .get(key)
        .ok_or_else(|| format!("Key '{}' not found in group '{}'", key, group))?;

    serde_json::from_str(value).map_err(|e| format!("Failed to deserialize value: {}", e))
}

/// Write a value to the store
///
/// # Arguments
///
/// * `app_name` - The application name
/// * `group` - The group name (e.g., "settings", "cache")
/// * `key` - The key to write
/// * `value` - The value to write (will be serialized to JSON)
///
/// # Errors
///
/// Returns an error if the value cannot be serialized or the file cannot be written.
///
/// # Example
///
/// ```ignore
/// use iced_store_plugin::{AppName, write_value};
/// use serde::Serialize;
///
/// #[derive(Serialize)]
/// struct UserPrefs {
///     theme: String,
///     font_size: u32,
/// }
///
/// async fn save_prefs() -> Result<(), String> {
///     let app_name = AppName::new("com", "example", "myapp");
///     let prefs = UserPrefs {
///         theme: "dark".to_string(),
///         font_size: 14,
///     };
///     write_value(&app_name, "settings", "user_prefs", &prefs).await
/// }
/// ```
pub async fn write_value<T>(
    app_name: &AppName,
    group: &str,
    key: &str,
    value: &T,
) -> Result<(), String>
where
    T: Serialize,
{
    let json_value =
        serde_json::to_string(value).map_err(|e| format!("Failed to serialize value: {}", e))?;

    modify_group(app_name, group, |data| {
        data.insert(key.to_string(), json_value);
        true
    })
    .await?;

    Ok(())
}

/// Delete a value from the store
///
/// # Arguments
///
/// * `app_name` - The application name
/// * `group` - The group name
/// * `key` - The key to delete
///
/// # Returns
///
/// Returns `Ok(true)` if the value was deleted, `Ok(false)` if it didn't exist.
///
/// # Errors
///
/// Returns an error if the group cannot be loaded or saved.
///
/// # Example
///
/// ```ignore
/// use iced_store_plugin::{AppName, delete_value};
///
/// async fn clear_cache() -> Result<bool, String> {
///     let app_name = AppName::new("com", "example", "myapp");
///     delete_value(&app_name, "cache", "temp_data").await
/// }
/// ```
pub async fn delete_value(app_name: &AppName, group: &str, key: &str) -> Result<bool, String> {
    modify_group(app_name, group, |data| data.remove(key).is_some()).await
}

/// Check if a key exists in the store
///
/// # Arguments
///
/// * `app_name` - The application name
/// * `group` - The group name
/// * `key` - The key to check
///
/// # Returns
///
/// Returns `Ok(true)` if the key exists, `Ok(false)` otherwise.
///
/// # Errors
///
/// Returns an error if the group cannot be loaded.
///
/// # Example
///
/// ```ignore
/// use iced_store_plugin::{AppName, has_value};
///
/// async fn check_first_run() -> bool {
///     let app_name = AppName::new("com", "example", "myapp");
///     !has_value(&app_name, "settings", "initialized")
///         .await
///         .unwrap_or(false)
/// }
/// ```
pub async fn has_value(app_name: &AppName, group: &str, key: &str) -> Result<bool, String> {
    let data = load_group(app_name, group).await?;
    Ok(data.contains_key(key))
}

/// List all keys in a group
///
/// # Arguments
///
/// * `app_name` - The application name
/// * `group` - The group name
///
/// # Returns
///
/// Returns a vector of all keys in the group (may be empty).
///
/// # Errors
///
/// Returns an error if the group cannot be loaded.
///
/// # Example
///
/// ```ignore
/// use iced_store_plugin::{AppName, list_keys};
///
/// async fn show_all_settings() -> Result<(), String> {
///     let app_name = AppName::new("com", "example", "myapp");
///     let keys = list_keys(&app_name, "settings").await?;
///     for key in keys {
///         println!("Setting: {}", key);
///     }
///     Ok(())
/// }
/// ```
pub async fn list_keys(app_name: &AppName, group: &str) -> Result<Vec<String>, String> {
    let data = load_group(app_name, group).await?;
    Ok(data.keys().cloned().collect())
}
