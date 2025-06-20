use std::borrow::Cow;
use std::sync::OnceLock;

use iced::font;

pub static MONO: Font = Font::new(false, false);
pub static MONO_BOLD: Font = Font::new(true, false);
pub static MONO_ITALICS: Font = Font::new(false, true);
pub static MONO_BOLD_ITALICS: Font = Font::new(true, true);
pub const ICON: iced::Font = iced::Font::with_name("editor-icons");

#[derive(Debug, Clone)]
pub struct Font {
    bold: bool,
    italics: bool,
    inner: OnceLock<iced::Font>,
}

impl Font {
    const fn new(bold: bool, italics: bool) -> Self {
        Self {
            bold,
            italics,
            inner: OnceLock::new(),
        }
    }

    fn set(&self, name: String, weight: font::Weight, bold_weight: font::Weight) {
        let name = Box::leak(name.into_boxed_str());
        let weight = if self.bold { bold_weight } else { weight };
        let style = if self.italics {
            font::Style::Italic
        } else {
            font::Style::Normal
        };

        let _ = self.inner.set(iced::Font {
            weight,
            style,
            ..iced::Font::with_name(name)
        });
    }
}

impl From<Font> for iced::Font {
    fn from(value: Font) -> Self {
        value.inner.get().copied().expect("font is set on startup")
    }
}

pub fn load() -> Vec<Cow<'static, [u8]>> {
    vec![include_bytes!("../fonts/icons.ttf").as_slice().into()]
}
