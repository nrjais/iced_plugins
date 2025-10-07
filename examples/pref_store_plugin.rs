//! Example demonstrating the Preference Store Plugin
//!
//! This example shows how to use the preference store plugin to persist
//! application data with type safety and group organization.

use iced::widget::{button, column, container, row, text, text_input};
use iced::{Element, Length, Task};
use iced_plugins::{PluginHandle, PluginManager, PluginMessage};
use iced_pref_store_plugin::{PrefMessage, PrefOutput, PrefStorePlugin};
use serde::{Deserialize, Serialize};

const APP_NAME: &str = "pref_store_example";

fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .subscription(subscription)
        .run()
}

/// User preferences structure
#[derive(Debug, Clone, Serialize, Deserialize)]
struct UserPrefs {
    theme: String,
    font_size: u32,
}

impl Default for UserPrefs {
    fn default() -> Self {
        Self {
            theme: "light".to_string(),
            font_size: 14,
        }
    }
}

/// Application state
struct App {
    plugins: PluginManager,
    pref_handle: PluginHandle<PrefStorePlugin>,

    // UI state
    user_prefs: UserPrefs,

    // Input fields
    theme_input: String,
    font_size_input: String,

    status_message: String,
}

#[derive(Debug, Clone)]
enum Message {
    Plugin(PluginMessage),
    PrefOutput(PrefOutput),

    // Preferences actions
    ThemeInputChanged(String),
    FontSizeInputChanged(String),
    SavePreferences,
    LoadPreferences,
    DeletePreferences,
}

impl App {
    fn new() -> (App, Task<Message>) {
        let mut builder = iced_plugins::PluginManagerBuilder::new();
        let pref_handle = builder.install(PrefStorePlugin::new(APP_NAME));
        let (plugins, init_task) = builder.build();

        let app = App {
            plugins,
            pref_handle: pref_handle.clone(),
            user_prefs: UserPrefs::default(),
            theme_input: "light".to_string(),
            font_size_input: "14".to_string(),
            status_message: "Ready".to_string(),
        };

        // Auto-load preferences on startup
        let load_task = pref_handle
            .dispatch(PrefMessage::get("ui", "user"))
            .map(Message::Plugin);

        (
            app,
            Task::batch([init_task.map(Message::Plugin), load_task]),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Plugin(plugin_msg) => {
                return self.plugins.update(plugin_msg).map(Message::Plugin);
            }

            Message::PrefOutput(output) => match output {
                PrefOutput::Get { ref key, .. } if key == "user" => {
                    if let Some(prefs) = output.as_value::<UserPrefs>() {
                        self.user_prefs = prefs.clone();
                        self.theme_input = prefs.theme;
                        self.font_size_input = prefs.font_size.to_string();
                        self.status_message = "Preferences loaded".to_string();
                    }
                }

                PrefOutput::Set { group, key } => {
                    self.status_message = format!("Saved {}/{}", group, key);
                }

                PrefOutput::Deleted { group, key } => {
                    self.status_message = format!("Deleted {}/{}", group, key);
                    // Reset to defaults
                    self.user_prefs = UserPrefs::default();
                    self.theme_input = self.user_prefs.theme.clone();
                    self.font_size_input = self.user_prefs.font_size.to_string();
                }

                PrefOutput::NotFound { key, .. } => {
                    self.status_message = format!("'{}' not found, using defaults", key);
                }

                PrefOutput::Error { message } => {
                    self.status_message = format!("Error: {}", message);
                }

                _ => {}
            },

            Message::ThemeInputChanged(value) => {
                self.theme_input = value;
            }

            Message::FontSizeInputChanged(value) => {
                self.font_size_input = value;
            }

            Message::SavePreferences => {
                if let Ok(font_size) = self.font_size_input.parse::<u32>() {
                    let prefs = UserPrefs {
                        theme: self.theme_input.clone(),
                        font_size,
                    };
                    self.user_prefs = prefs.clone();

                    return self
                        .pref_handle
                        .dispatch(PrefMessage::set("ui", "user", prefs))
                        .map(Message::Plugin);
                } else {
                    self.status_message = "Invalid font size".to_string();
                }
            }

            Message::LoadPreferences => {
                return self
                    .pref_handle
                    .dispatch(PrefMessage::get("ui", "user"))
                    .map(Message::Plugin);
            }

            Message::DeletePreferences => {
                return self
                    .pref_handle
                    .dispatch(PrefMessage::delete("ui", "user"))
                    .map(Message::Plugin);
            }
        }

        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let title = text("Preference Store Plugin Example").size(32);

        let status = text(format!("Status: {}", self.status_message)).size(14);

        // Current preferences display
        let current_prefs = text(format!(
            "Current: theme={}, font_size={}",
            self.user_prefs.theme, self.user_prefs.font_size
        ));

        // Theme input
        let theme_input = row![
            text("Theme:").width(100),
            text_input("light, dark, auto", &self.theme_input)
                .on_input(Message::ThemeInputChanged)
                .width(200),
        ]
        .spacing(10);

        // Font size input
        let font_size_input = row![
            text("Font Size:").width(100),
            text_input("14", &self.font_size_input)
                .on_input(Message::FontSizeInputChanged)
                .width(200),
        ]
        .spacing(10);

        // Action buttons
        let buttons = row![
            button("Save").on_press(Message::SavePreferences),
            button("Load").on_press(Message::LoadPreferences),
            button("Delete").on_press(Message::DeletePreferences),
        ]
        .spacing(10);

        let content = column![
            title,
            status,
            text("").size(10),
            current_prefs,
            text("").size(10),
            theme_input,
            font_size_input,
            text("").size(10),
            buttons,
        ]
        .spacing(10)
        .padding(20);

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

fn subscription(app: &App) -> iced::Subscription<Message> {
    iced::Subscription::batch([
        app.plugins.subscriptions().map(Message::Plugin),
        app.pref_handle.listen().map(Message::PrefOutput),
    ])
}
