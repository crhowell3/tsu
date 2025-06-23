use std::path::PathBuf;
use std::time::Duration;

use data::Config;
use iced::Length::*;
use iced::alignment::Vertical;
use iced::widget::text::LineHeight;
use iced::widget::{button, center, column, container, row, text_input};
use iced::{Color, Length, Task, Vector, alignment, clipboard};
use tokio::time;

use crate::theme::{self, Colors, Theme};
use crate::widget::{Element, color_picker, combo_box, tooltip};
use crate::window::{self, Window};
use crate::{icon, widget};

#[derive(Debug, Clone)]
pub enum Event {
    Close,
    ReloadThemes,
}

#[derive(Debug, Clone)]
pub enum Message {
    Color(Color),
    Component(Component),
    HexInput(String),
    Save,
    Apply,
    Discard,
    Revert,
    Clear,
    Copy,
    Share,
    SavePath(Option<PathBuf>),
    Saved(Result<(), String>),
    ClearSaveResult,
    ClearCopy,
}

#[derive(Debug, Clone)]
pub struct ThemeEditor {
    pub window: window::Id,
    combo_box: combo_box::State<Component>,
    component: Component,
    hex_input: Option<String>,
    save_result: Option<bool>,
    copied: bool,
}

impl ThemeEditor {
    pub fn open(main_window: &Window) -> (Self, Task<window::Id>) {
        let (window, task) = window::open(window::Settings {
            size: iced::Size::new(470.0, 300.0),
            resizable: false,
            position: main_window
                .position
                .map(|point| window::Position::Specific(point + Vector::new(20.0, 20.0)))
                .unwrap_or_default(),
            exit_on_close_request: false,
            ..window::settings()
        });

        (
            Self {
                window,
                combo_box: combo_box::State::new(components().collect()),
                component: Component::Text(Text::Primary),
                hex_input: None,
                save_result: None,
                copied: false,
            },
            task,
        )
    }
}

impl ThemeEditor {
    pub fn update(
        &mut self,
        message: Message,
        theme: &mut Theme,
    ) -> (Task<Message>, Option<Event>) {
        match message {
            Message::Color(color) => {
                self.hex_input = None;

                let mut colors = *theme.colors();

                self.component.update(&mut colors, Some(color));

                *theme = theme.preview(data::Theme::new("Custom Theme".into(), colors));
            }
            Message::Component(component) => {
                self.hex_input = None;
                self.combo_box = combo_box::State::new(components().collect());

                self.component = component;
            }
            Message::HexInput(input) => {
                if let Some(color) = theme::hex_to_color(&input) {
                    let mut colors = *theme.colors();

                    self.component.update(&mut colors, Some(color));

                    *theme = theme.preview(data::Theme::new("Custom Theme".into(), colors));
                }

                self.hex_input = Some(input);
            }
            Message::Save => {
                let task = async move {
                    rfd::AsyncFileDialog::new()
                        .set_directory(Config::themes_dir())
                        .set_file_name("custom-theme.toml")
                        .save_file()
                        .await
                        .map(|handle| handle.path().to_path_buf())
                };

                return (Task::perform(task, Message::SavePath), None);
            }
            Message::Apply => {
                return (Task::none(), Some(Event::Close));
            }
            Message::Discard => {
                *theme = theme.selected();

                return (Task::none(), Some(Event::Close));
            }
            Message::Revert => {
                self.hex_input = None;

                let mut colors = *theme.selected().colors();
                let original = self.component.color(&colors);

                self.component.update(&mut colors, original);

                *theme = theme.preview(data::Theme::new("Custom Theme".into(), colors));
            }
            Message::Clear => {
                self.hex_input = None;

                let mut colors = *theme.colors();

                self.component.update(&mut colors, None);

                *theme = theme.preview(data::Theme::new("Custom Theme".into(), colors));
            }
            Message::Copy => {
                self.copied = true;

                let url = url::theme(theme.colors());

                return (
                    Task::batch(vec![
                        clipboard::write(url),
                        Task::perform(time::sleep(Duration::from_secs(2)), |()| Message::ClearCopy),
                    ]),
                    None,
                );
            }
            Message::Share => {
                let url = url::theme_submit(theme.colors());
                let _ = open::that_detached(url);

                return (Task::none(), None);
            }
            Message::SavePath(None) => {}
            Message::SavePath(Some(path)) => {
                debug!("Saving theme to {path:?}");

                let colors = *theme.colors();

                return (
                    Task::perform(colors.save(path).map_err(|e| e.to_string()), Message::Saved),
                    None,
                );
            }
            Message::Saved(Err(err)) => {
                error!("Failed to save theme: {err}");
                self.save_result = Some(false);

                return (
                    Task::perform(time::sleep(Duration::from_secs(2)), |()| {
                        Message::ClearSaveResult
                    }),
                    None,
                );
            }
            Message::Saved(Ok(())) => {
                debug!("Theme saved");
                self.save_result = Some(true);

                return (
                    Task::perform(time::sleep(Duration::from_secs(2)), |()| {
                        Message::ClearSaveResult
                    }),
                    Some(Event::ReloadThemes),
                );
            }
            Message::ClearSaveResult => {
                self.save_result = None;
            }
            Message::ClearCopy => {
                self.copied = false;
            }
        }

        (Task::none(), None)
    }

    pub fn view<'a>(&'a self, theme: &'a Theme) -> Element<'a, Message> {
        let color = self
            .component
            .color(theme.colors())
            .unwrap_or(Color::TRANSPARENT);

        let component = combo_box(
            &self.combo_box,
            &self.component.to_string(),
            None,
            Message::Component,
        );

        let is_input_valid = self.hex_input.is_none()
            || self
                .hex_input
                .as_deref()
                .and_then(theme::hex_to_color)
                .is_some();
        let hex_input = text_input(
            "",
            self.hex_input
                .as_deref()
                .unwrap_or(theme::color_to_hex(color).as_str()),
        )
        .on_input(Message::HexInput)
        .style(move |theme, status| {
            if is_input_valid {
                theme::text_input::primary(theme, status)
            } else {
                theme::text_input::error(theme, status)
            }
        });

        let undo = icon(icon::undo(), "Revert Color", Message::Revert);

        let save = match self.save_result {
            Some(is_success) => status_button(is_success),
            None => secondary_button("Save to Disk", Message::Save),
        };
        let apply = secondary_button("Apply Colors", Message::Apply);

        let copy = if self.copied {
            success_icon()
        } else {
            icon(icon::copy(), "Copy Theme to URL", Message::Copy)
        };

        let color_picker = color_picker(color, Message::Color);

        let content = column![
            row![
                container(component).width(Fill),
                container(hex_input).width(80),
                undo,
                copy,
            ]
            .align_y(Vertical::Center)
            .spacing(4),
            color_picker,
            row![apply, save].spacing(4),
        ]
        .spacing(8);

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(8)
            .style(theme::container::general)
            .into()
    }
}
