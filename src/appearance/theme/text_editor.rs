use iced::widget::text_editor::{Catalog, Status, Style, StyleFn};
use iced::{Background, Border, Color};

use super::Theme;

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

    let active = Style {
        background: iced::Background::Color(general.background),
        border: iced::Border {
            color: general.border,
            width: 1.0,
            radius: 0.0.into(),
        },
        icon: theme.colors().text.primary,
        placeholder: theme.colors().text.secondary,
        value: theme.colors().text.primary,
        selection: theme.colors().general.background,
    };

    match status {
        Status::Active | Status::Hovered | Status::Focused { .. } => active,
        Status::Disabled => Style {
            background: Background::Color(theme.colors().general.background),
            placeholder: Color {
                a: 0.2,
                ..theme.colors().text.secondary
            },
            border: Border {
                radius: 4.0.into(),
                width: 1.0,
                color: theme.colors().general.border,
            },
            ..active
        },
    }
}
