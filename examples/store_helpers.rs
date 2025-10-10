//! Example demonstrating standalone store helpers (no plugin initialization required)
//!
//! This example shows how to use the store helpers directly without
//! initializing the plugin system. Useful for CLI tools and scripts.

use iced_store_plugin::{AppName, delete_value, has_value, list_keys, read_value, write_value};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AppConfig {
    theme: String,
    font_size: u32,
    auto_save: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            theme: "light".to_string(),
            font_size: 14,
            auto_save: true,
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), String> {
    let app_name = AppName::new("com", "nrjais", "store_plugin");

    println!("=== Store Helpers Example ===\n");

    // 1. Write a value
    println!("1. Writing config...");
    let config = AppConfig {
        theme: "dark".to_string(),
        font_size: 16,
        auto_save: true,
    };
    write_value(&app_name, "settings", "config", &config).await?;
    println!("   ✓ Config written: {:?}\n", config);

    // 2. Read the value back
    println!("2. Reading config...");
    let loaded_config: AppConfig = read_value(&app_name, "settings", "config").await?;
    println!("   ✓ Config loaded: {:?}\n", loaded_config);

    // 3. Check if a key exists
    println!("3. Checking if key exists...");
    let exists = has_value(&app_name, "settings", "config").await?;
    println!("   ✓ Key 'config' exists: {}\n", exists);

    // 4. Write multiple values
    println!("4. Writing multiple values...");
    write_value(&app_name, "settings", "language", &"en-US".to_string()).await?;
    write_value(&app_name, "settings", "notifications", &true).await?;
    println!("   ✓ Multiple values written\n");

    // 5. List all keys in a group
    println!("5. Listing all keys in 'settings' group...");
    let keys = list_keys(&app_name, "settings").await?;
    println!("   ✓ Keys: {:?}\n", keys);

    // 6. Read a specific value
    println!("6. Reading language setting...");
    let language: String = read_value(&app_name, "settings", "language").await?;
    println!("   ✓ Language: {}\n", language);

    // 7. Update a value
    println!("7. Updating config...");
    let mut updated_config = loaded_config;
    updated_config.font_size = 18;
    write_value(&app_name, "settings", "config", &updated_config).await?;
    println!("   ✓ Config updated: {:?}\n", updated_config);

    // 8. Delete a value
    println!("8. Deleting language setting...");
    let was_deleted = delete_value(&app_name, "settings", "language").await?;
    println!("   ✓ Deleted: {}\n", was_deleted);

    // 9. Try to read a non-existent value
    println!("9. Trying to read deleted value...");
    match read_value::<String>(&app_name, "settings", "language").await {
        Ok(value) => println!("   ✗ Unexpected value: {}", value),
        Err(e) => println!("   ✓ Expected error: {}\n", e),
    }

    // 10. List keys again to verify deletion
    println!("10. Listing keys after deletion...");
    let keys_after = list_keys(&app_name, "settings").await?;
    println!("    ✓ Keys: {:?}\n", keys_after);

    println!("=== Example Complete ===");

    Ok(())
}
