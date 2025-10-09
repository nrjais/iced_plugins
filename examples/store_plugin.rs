//! Example demonstrating the Store Plugin
//!
//! This example shows how to use the store plugin to persist
//! application data with group organization.

use iced::widget::{button, column, row, scrollable, text, text_input};
use iced::{Element, Length, Task};
use iced_plugins::{PluginHandle, PluginManager, PluginMessage};
use iced_store_plugin::{AppName, StoreInput, StoreOutput, StorePlugin};
use serde::{Deserialize, Serialize};

fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .subscription(subscription)
        .run()
}

/// User data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
struct UserData {
    theme: String,
    font_size: u32,
}

impl Default for UserData {
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
    store_handle: PluginHandle<StorePlugin>,

    // UI state
    user_data: UserData,

    // Input fields
    theme_input: String,
    font_size_input: String,

    status_message: String,
}

#[derive(Debug, Clone)]
enum Message {
    Plugin(PluginMessage),
    StoreOutput(StoreOutput),

    // Data actions
    ThemeInputChanged(String),
    FontSizeInputChanged(String),
    SaveData,
    LoadData,
    DeleteData,
}

impl App {
    fn new() -> (App, Task<Message>) {
        let app_name = AppName::new("com", "nrjais", "store_plugin");
        let mut builder = iced_plugins::PluginManagerBuilder::new();
        let store_handle = builder.install(StorePlugin::new(app_name));
        let (plugins, init_task) = builder.build();

        let app = App {
            plugins,
            store_handle: store_handle.clone(),
            user_data: UserData::default(),
            theme_input: "light".to_string(),
            font_size_input: "14".to_string(),
            status_message: "Ready".to_string(),
        };

        // Auto-load data on startup
        let load_task = store_handle
            .dispatch(StoreInput::get("ui", "user"))
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

            Message::StoreOutput(output) => match output {
                StoreOutput::Get { ref key, .. } if key == "user" => {
                    if let Some(data) = output.as_value::<UserData>() {
                        self.user_data = data.clone();
                        self.theme_input = data.theme;
                        self.font_size_input = data.font_size.to_string();
                        self.status_message = "Data loaded".to_string();
                    }
                }

                StoreOutput::Set { group, key } => {
                    self.status_message = format!("Saved {}/{}", group, key);
                }

                StoreOutput::Deleted { group, key } => {
                    self.status_message = format!("Deleted {}/{}", group, key);
                    // Reset to defaults
                    self.user_data = UserData::default();
                    self.theme_input = self.user_data.theme.clone();
                    self.font_size_input = self.user_data.font_size.to_string();
                }

                StoreOutput::NotFound { key, .. } => {
                    self.status_message = format!("'{}' not found, using defaults", key);
                }

                StoreOutput::Error { message } => {
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

            Message::SaveData => {
                if let Ok(font_size) = self.font_size_input.parse::<u32>() {
                    let data = UserData {
                        theme: self.theme_input.clone(),
                        font_size,
                    };
                    self.user_data = data.clone();

                    return self
                        .store_handle
                        .dispatch(StoreInput::set("ui", "user", data))
                        .map(Message::Plugin);
                } else {
                    self.status_message = "Invalid font size".to_string();
                }
            }

            Message::LoadData => {
                return self
                    .store_handle
                    .dispatch(StoreInput::get("ui", "user"))
                    .map(Message::Plugin);
            }

            Message::DeleteData => {
                return self
                    .store_handle
                    .dispatch(StoreInput::delete("ui", "user"))
                    .map(Message::Plugin);
            }
        }

        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let title = text("Store Plugin Example").size(32);

        let status = text(format!("Status: {}", self.status_message)).size(14);

        // Current data display
        let current_data = text(format!(
            "Current: theme={}, font_size={}",
            self.user_data.theme, self.user_data.font_size
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
            button("Save").on_press(Message::SaveData),
            button("Load").on_press(Message::LoadData),
            button("Delete").on_press(Message::DeleteData),
        ]
        .spacing(10);

        let content = column![
            title,
            status,
            text("").size(10),
            current_data,
            text("").size(10),
            theme_input,
            font_size_input,
            text("").size(10),
            buttons,
        ]
        .spacing(10)
        .padding(20);

        scrollable(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

fn subscription(app: &App) -> iced::Subscription<Message> {
    iced::Subscription::batch([
        app.plugins.subscriptions().map(Message::Plugin),
        app.store_handle.listen().map(Message::StoreOutput),
    ])
}
