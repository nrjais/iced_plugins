use std::{collections::HashMap, sync::Arc};
use tray_icon::Icon;
use tray_icon::menu::{
    CheckMenuItem as TrayCheckMenuItem, Menu as TrayMenu, MenuId, MenuItem as TrayMenuItem,
    PredefinedMenuItem, Submenu as TraySubmenu,
};

/// Menu builder that constructs menu items with stored state
#[derive(Clone, Debug)]
pub struct Menu {
    items: Vec<MenuItem>,
}

impl Menu {
    /// Create a new menu
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    /// Add a menu item
    pub fn add_item(&mut self, item: MenuItem) {
        self.items.push(item);
    }

    /// Get all items
    pub fn items(&self) -> &[MenuItem] {
        &self.items
    }
}

impl Default for Menu {
    fn default() -> Self {
        Self::new()
    }
}

/// Menu item types - stores only the data, native items are created separately
#[derive(Clone, Debug)]
pub enum MenuItem {
    /// Regular menu item
    Item {
        id: String,
        text: String,
        enabled: bool,
    },
    /// Checkable menu item
    CheckItem {
        id: String,
        text: String,
        enabled: bool,
        checked: bool,
    },
    /// Submenu
    Submenu {
        id: String,
        text: String,
        enabled: bool,
        items: Vec<MenuItem>,
    },
    /// Separator
    Separator,
}

impl MenuItem {
    /// Create a new menu item
    pub fn new(id: impl Into<String>, text: impl Into<String>, enabled: bool) -> Self {
        Self::Item {
            id: id.into(),
            text: text.into(),
            enabled,
        }
    }

    /// Create a new checkable menu item
    pub fn new_check(
        id: impl Into<String>,
        text: impl Into<String>,
        enabled: bool,
        checked: bool,
    ) -> Self {
        Self::CheckItem {
            id: id.into(),
            text: text.into(),
            enabled,
            checked,
        }
    }

    /// Create a new submenu
    pub fn new_submenu(
        id: impl Into<String>,
        text: impl Into<String>,
        enabled: bool,
        items: Vec<MenuItem>,
    ) -> Self {
        Self::Submenu {
            id: id.into(),
            text: text.into(),
            enabled,
            items,
        }
    }

    /// Create a separator
    pub fn separator() -> Self {
        Self::Separator
    }

    /// Get the menu item ID
    pub fn id(&self) -> Option<&str> {
        match self {
            Self::Item { id, .. } | Self::CheckItem { id, .. } | Self::Submenu { id, .. } => {
                Some(id)
            }
            Self::Separator => None,
        }
    }
}

// Wrapper for native menu items to make them Send
pub struct NativeMenuItem {
    kind: NativeMenuItemKind,
}

pub enum NativeMenuItemKind {
    Item(TrayMenuItem),
    CheckItem(TrayCheckMenuItem),
    Submenu(TraySubmenu),
}

impl NativeMenuItem {
    fn new_item(id: &str, text: &str, enabled: bool) -> Self {
        Self {
            kind: NativeMenuItemKind::Item(TrayMenuItem::with_id(
                MenuId::new(id),
                text,
                enabled,
                None,
            )),
        }
    }

    fn new_check_item(id: &str, text: &str, enabled: bool, checked: bool) -> Self {
        Self {
            kind: NativeMenuItemKind::CheckItem(TrayCheckMenuItem::with_id(
                MenuId::new(id),
                text,
                enabled,
                checked,
                None,
            )),
        }
    }

    fn new_submenu(id: &str, text: &str, enabled: bool) -> Self {
        Self {
            kind: NativeMenuItemKind::Submenu(TraySubmenu::with_id(MenuId::new(id), text, enabled)),
        }
    }

    fn append_to_menu(&self, menu: &TrayMenu) -> Result<(), String> {
        match &self.kind {
            NativeMenuItemKind::Item(item) => menu
                .append(item)
                .map_err(|e| format!("Failed to append item: {}", e)),
            NativeMenuItemKind::CheckItem(item) => menu
                .append(item)
                .map_err(|e| format!("Failed to append check item: {}", e)),
            NativeMenuItemKind::Submenu(submenu) => menu
                .append(submenu)
                .map_err(|e| format!("Failed to append submenu: {}", e)),
        }
    }

    fn append_to_submenu(&self, submenu: &TraySubmenu) -> Result<(), String> {
        match &self.kind {
            NativeMenuItemKind::Item(item) => submenu
                .append(item)
                .map_err(|e| format!("Failed to append item: {}", e)),
            NativeMenuItemKind::CheckItem(item) => submenu
                .append(item)
                .map_err(|e| format!("Failed to append check item: {}", e)),
            NativeMenuItemKind::Submenu(sub) => submenu
                .append(sub)
                .map_err(|e| format!("Failed to append submenu: {}", e)),
        }
    }

    fn submenu(&self) -> Option<&TraySubmenu> {
        match &self.kind {
            NativeMenuItemKind::Submenu(sub) => Some(sub),
            _ => None,
        }
    }

    fn update_from_item(&self, item: &MenuItem) {
        match (&self.kind, item) {
            (NativeMenuItemKind::Item(native), MenuItem::Item { text, enabled, .. }) => {
                native.set_text(text);
                native.set_enabled(*enabled);
            }
            (
                NativeMenuItemKind::CheckItem(native),
                MenuItem::CheckItem {
                    text,
                    enabled,
                    checked,
                    ..
                },
            ) => {
                native.set_text(text);
                native.set_enabled(*enabled);
                native.set_checked(*checked);
            }
            (NativeMenuItemKind::Submenu(native), MenuItem::Submenu { text, enabled, .. }) => {
                native.set_text(text);
                native.set_enabled(*enabled);
            }
            _ => {}
        }
    }
}

// SAFETY: We control access through the plugin system
unsafe impl Send for NativeMenuItem {}
unsafe impl Sync for NativeMenuItem {}

/// Create an icon from bytes
pub fn create_icon(bytes: &[u8]) -> Result<Icon, String> {
    let image =
        image::load_from_memory(bytes).map_err(|e| format!("Failed to load icon image: {}", e))?;

    let rgba = image.to_rgba8();
    let (width, height) = rgba.dimensions();

    Icon::from_rgba(rgba.into_raw(), width, height)
        .map_err(|e| format!("Failed to create icon: {}", e))
}

/// Build native menu items from menu structure
fn build_native_items(
    item: &MenuItem,
    native_items: &mut HashMap<String, Arc<NativeMenuItem>>,
) -> Arc<NativeMenuItem> {
    match item {
        MenuItem::Item { id, text, enabled } => {
            let native = Arc::new(NativeMenuItem::new_item(id, text, *enabled));
            native_items.insert(id.clone(), Arc::clone(&native));
            native
        }
        MenuItem::CheckItem {
            id,
            text,
            enabled,
            checked,
        } => {
            let native = Arc::new(NativeMenuItem::new_check_item(id, text, *enabled, *checked));
            native_items.insert(id.clone(), Arc::clone(&native));
            native
        }
        MenuItem::Submenu {
            id,
            text,
            enabled,
            items,
        } => {
            let native = Arc::new(NativeMenuItem::new_submenu(id, text, *enabled));
            native_items.insert(id.clone(), Arc::clone(&native));

            // Recursively build submenu items
            if let Some(submenu) = native.submenu() {
                for child in items {
                    match child {
                        MenuItem::Separator => {
                            let _ = submenu.append(&PredefinedMenuItem::separator());
                        }
                        _ => {
                            let child_native = build_native_items(child, native_items);
                            let _ = child_native.append_to_submenu(submenu);
                        }
                    }
                }
            }

            native
        }
        MenuItem::Separator => {
            // Separators don't have IDs or state
            Arc::new(NativeMenuItem::new_item("", "", false)) // Placeholder, won't be stored
        }
    }
}

/// Build native menu and collect native items
pub fn build_native_menu(menu: &Menu) -> (TrayMenu, HashMap<String, Arc<NativeMenuItem>>) {
    let native_menu = TrayMenu::new();
    let mut native_items = HashMap::new();

    for item in menu.items() {
        match item {
            MenuItem::Separator => {
                let _ = native_menu.append(&PredefinedMenuItem::separator());
            }
            _ => {
                let native_item = build_native_items(item, &mut native_items);
                let _ = native_item.append_to_menu(&native_menu);
            }
        }
    }

    (native_menu, native_items)
}

/// Recursively update menu items
pub fn update_menu_items(item: &MenuItem, native_items: &HashMap<String, Arc<NativeMenuItem>>) {
    if let Some(id) = item.id()
        && let Some(native) = native_items.get(id)
    {
        native.update_from_item(item);
    }

    if let MenuItem::Submenu { items, .. } = item {
        for child in items {
            update_menu_items(child, native_items);
        }
    }
}
