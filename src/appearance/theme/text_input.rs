use iced::widget::text_input::{Catalog, Status, Style, StyleFn};
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
    let active = Style {
        background: Background::Color(theme.colors().general.background),
        border: Border {
            radius: 4.0.into(),
            width: 1.0,
            color: theme.colors().general.border,
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

pub fn error(theme: &Theme, status: Status) -> Style {
    let primary = primary(theme, status);

    match status {
        Status::Active | Status::Hovered | Status::Focused { .. } => Style {
            border: Border {
                radius: 4.0.into(),
                width: 1.0,
                color: theme.colors().text.error,
            },
            ..primary
        },
        Status::Disabled => primary,
    }
}
