use super::Theme;
use iced::widget::text_editor::{Catalog, Status, Style, StyleFn};

impl Catalog for Theme {
    type Class<'a> = StyleFn<'a, Self>;

    fn default<'a>() -> Self::Class<'a> {
        Box::new(primary)
    }

    fn style(&self, class: &Self::Class<'_>, status: Status) -> Style {
        class(self, status)
    }
}

pub fn primary(theme: &Theme, status: Status) -> Style {
    let general = theme.colors().general;

    Style {
        background: iced::Background::Color(general.background),
        border: iced::Border {
            color: iced::Color::TRANSPARENT,
            width: 0.0,
            radius: 0.0.into(),
        },
        icon: iced::Color::WHITE,
        placeholder: iced::Color::WHITE,
        selection: iced::Color::WHITE,
        value: iced::Color::WHITE,
    }
}
