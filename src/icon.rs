use iced::widget::text;
use iced::widget::text::LineHeight;

use crate::widget::Text;
use crate::{font, theme};

pub fn new_icon<'a>() -> Text<'a> {
    to_text('\u{0e800}')
}

pub fn save_icon<'a>() -> Text<'a> {
    to_text('\u{0e801}')
}

pub fn open_icon<'a>() -> Text<'a> {
    to_text('\u{0f115}')
}

fn to_text<'a>(unicode: char) -> Text<'a> {
    text(unicode.to_string())
        .line_height(LineHeight::Relative(1.0))
        .size(theme::ICON_SIZE)
        .font(font::ICON)
}
