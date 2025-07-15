use iced::widget::text;
use iced::widget::text::LineHeight;

use crate::widget::Text;
use crate::{font, theme};

pub fn dot<'a>() -> Text<'a> {
    to_text('\u{F111}')
}

pub fn error<'a>() -> Text<'a> {
    to_text('\u{E80D}')
}

pub fn new_icon<'a>() -> Text<'a> {
    to_text('\u{0e800}')
}

pub fn save_icon<'a>() -> Text<'a> {
    to_text('\u{0e801}')
}

pub fn open_icon<'a>() -> Text<'a> {
    to_text('\u{0f115}')
}

pub fn maximize<'a>() -> Text<'a> {
    to_text('\u{E801}')
}

pub fn restore<'a>() -> Text<'a> {
    to_text('\u{E805}')
}

pub fn checkmark<'a>() -> Text<'a> {
    to_text('\u{E806}')
}

pub fn theme_editor<'a>() -> Text<'a> {
    to_text('\u{E80A}')
}

pub fn undo<'a>() -> Text<'a> {
    to_text('\u{E80B}')
}

pub fn copy<'a>() -> Text<'a> {
    to_text('\u{F0C5}')
}

pub fn popout<'a>() -> Text<'a> {
    to_text('\u{E80E}')
}

pub fn config<'a>() -> Text<'a> {
    to_text('\u{F1C9}')
}

fn to_text<'a>(unicode: char) -> Text<'a> {
    text(unicode.to_string())
        .line_height(LineHeight::Relative(1.0))
        .size(theme::ICON_SIZE)
        .font(font::ICON)
}
