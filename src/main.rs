use iced::highlighter;
use iced::keyboard;
use iced::widget::{self, button, text, text_editor};
use iced::{Center, Element, Fill, Font, Task, Theme};

use std::ffi;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub fn main() -> iced::Result {
    iced::application(Tsu::new, Tsu::update, Tsu::view)
        .theme(Tsu::theme)
        .run()
}

struct Tsu {
    file: Option<PathBuf>,
    content: text_editor::Content,
    theme: highlighter::None,
    word_wrap: bool,
    is_loading: bool,
    is_dirty: bool,
}

#[derive(Debug, Clone)]
enum Message {
    ActionPerformed(text_editor::Action),
    ThemeSelected(highlighter::Theme),
    WordWrapToggled(bool),
    NewFile,
    OpenFile,
    FileOpened(Result<(PathBuf, Arc<String>), Error>),
    SaveFile,
    FileSaved(Result<PathBuf, Error>),
}

impl Tsu {
    fn new() -> (Self, Task<Message>) {
        (
            Self {
                file: None,
                content: text_editor::Content::new(),
                theme: highlighter::Theme::SolarizedDark,
                word_wrap: true,
                is_loading: true,
                is_dirty: false,
            },
            Task::batch([
                Task::perform(
                    load_file(format!("{}/src/main.rs", env!("CARGO_MANIFEST_DIR"))),
                    Message::FileOpened,
                ),
                widget::focus_next(),
            ]),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {}

    fn view(&self) -> Element<Message> {}

    fn theme(&self) -> Theme {
        if self.theme.is_dark() {
            Theme::Dark
        } else {
            Theme::Light
        }
    }
}

#[derive(Debug, Clone)]
pub enum Error {
    DialogClosed,
    IoError(io::ErrorKind),
}

async fn open_file() -> Result<(PathBuf, Arc<String>), Error> {}

fn new_icon<'a, Message>() -> Element<'a, Message> {
    icon('\u{0e800}')
}

fn save_icon<'a, Message>() -> Element<'a, Message> {
    icon('\u{0e801}')
}

fn open_icon<'a, Message>() -> Element<'a, Message> {
    icon('\u{0f115}')
}

fn icon<'a, Message>(codepoint: char) -> Element<'a, Message> {
    const ICON_FONT: Font = Font::with_name("editor-icons");

    text(codepoint).font(ICON_FONT).into()
}
