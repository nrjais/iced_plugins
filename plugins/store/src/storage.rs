//! Storage operations for persisting data to disk

use crate::app_name::AppName;
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::fs;

/// Get the storage directory for the application
///
/// Uses platform-specific conventions:
/// - Linux: `$XDG_CONFIG_HOME/<app>/store` or `~/.config/<app>/store`
/// - macOS: `~/Library/Application Support/<app>/store`
/// - Windows: `%APPDATA%\<app>\store`
pub fn storage_dir(app_name: &AppName) -> PathBuf {
    directories::ProjectDirs::from(
        app_name.qualifier.as_str(),
        app_name.organization.as_str(),
        app_name.application.as_str(),
    )
    .map(|dirs| dirs.config_local_dir().to_path_buf())
    .unwrap_or_else(|| PathBuf::from("."))
    .join("store")
}

/// Get the file path for a specific group
///
/// Each group is stored in a separate JSON file named `<group>.json`
pub fn get_group_path(app_name: &AppName, group: &str) -> PathBuf {
    storage_dir(app_name).join(format!("{}.json", group))
}

/// Load a group from disk
///
/// Returns an empty HashMap if the file doesn't exist or is empty.
///
/// # Errors
///
/// Returns an error if the file cannot be read or parsed.
pub async fn load_group(
    app_name: &AppName,
    group: &str,
) -> Result<HashMap<String, String>, String> {
    let path = get_group_path(app_name, group);

    if !path.exists() {
        return Ok(HashMap::new());
    }

    let contents = fs::read_to_string(&path)
        .await
        .map_err(|e| format!("Failed to read group file: {}", e))?;

    if contents.is_empty() {
        return Ok(HashMap::new());
    }

    serde_json::from_str(&contents).map_err(|e| format!("Failed to parse group file: {}", e))
}

/// Save a group to disk
///
/// Creates the storage directory if it doesn't exist.
/// The data is saved as pretty-printed JSON.
///
/// # Errors
///
/// Returns an error if the directory cannot be created, the data cannot be
/// serialized, or the file cannot be written.
pub async fn save_group(
    app_name: &AppName,
    group: &str,
    data: HashMap<String, String>,
) -> Result<(), String> {
    let path = get_group_path(app_name, group);

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .await
            .map_err(|e| format!("Failed to create storage directory: {}", e))?;
    }

    let contents = serde_json::to_string_pretty(&data)
        .map_err(|e| format!("Failed to serialize group: {}", e))?;

    fs::write(&path, contents)
        .await
        .map_err(|e| format!("Failed to write group file: {}", e))?;

    Ok(())
}

/// Modify a group by loading it, applying a modification function, and saving it back
///
/// The modifier function receives a mutable reference to the group data and should
/// return `true` if the data was modified, `false` otherwise. The data is only saved
/// if the modifier returns `true`.
///
/// # Arguments
///
/// * `app_name` - The application name
/// * `group` - The group name
/// * `modifier` - A function that modifies the group data
///
/// # Returns
///
/// Returns `Ok(true)` if the data was modified and saved, `Ok(false)` if not modified.
///
/// # Errors
///
/// Returns an error if loading or saving fails.
pub async fn modify_group<F>(app_name: &AppName, group: &str, modifier: F) -> Result<bool, String>
where
    F: FnOnce(&mut HashMap<String, String>) -> bool,
{
    let mut data = load_group(app_name, group).await?;
    let modified = modifier(&mut data);

    if modified {
        save_group(app_name, group, data).await?;
    }

    Ok(modified)
}
