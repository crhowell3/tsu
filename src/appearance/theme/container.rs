use iced::widget::container::{Catalog, Style, StyleFn, transparent};
use iced::{Background, Border, Color, border};

use super::Theme;

impl Catalog for Theme {
    type Class<'a> = StyleFn<'a, Self>;

    fn default<'a>() -> Self::Class<'a> {
        Box::new(transparent)
    }

    fn style(&self, class: &Self::Class<'_>) -> Style {
        class(self)
    }
}

pub fn buffer(theme: &Theme, selected: bool) -> Style {
    let buffer = theme.colors().general;

    Style {
        background: Some(Background::Color(buffer.background)),
        border: Border {
            radius: 4.0.into(),
            width: 1.0,
            color: if selected {
                buffer.background
            } else {
                buffer.border
            },
        },
        ..Default::default()
    }
}

pub fn buffer_title_bar(theme: &Theme) -> Style {
    let colors = theme.colors().general;

    Style {
        background: Some(Background::Color(colors.background)),
        text_color: Some(theme.colors().text.secondary),
        border: Border {
            radius: border::top_left(4).top_right(4),
            width: 1.0,
            color: Color::TRANSPARENT,
        },
        ..Default::default()
    }
}

pub fn table(theme: &Theme, idx: usize) -> Style {
    let general = theme.colors().general;
    let buffer = theme.colors().general;

    let background = if idx % 2 != 0 {
        general.background
    } else {
        buffer.background
    };

    Style {
        background: Some(Background::Color(background)),
        text_color: Some(theme.colors().text.primary),
        ..Default::default()
    }
}

pub fn none(_theme: &Theme) -> Style {
    Style {
        background: None,
        ..Default::default()
    }
}

pub fn primary_background_hover(theme: &Theme) -> Style {
    Style {
        background: Some(Background::Color(
            theme.colors().buttons.primary.background_hover,
        )),
        border: Border {
            radius: 4.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

pub fn general(theme: &Theme) -> Style {
    Style {
        background: Some(Background::Color(theme.colors().general.background)),
        text_color: Some(theme.colors().text.primary),
        ..Default::default()
    }
}

pub fn image_card(theme: &Theme) -> Style {
    let general = theme.colors().general;

    Style {
        background: Some(Background::Color(general.background)),
        border: Border {
            radius: 4.0.into(),
            width: 1.0,
            color: general.border,
        },
        ..Default::default()
    }
}

pub fn tooltip(theme: &Theme) -> Style {
    let general = theme.colors().general;

    Style {
        background: Some(Background::Color(general.background)),
        border: Border {
            radius: 4.0.into(),
            width: 1.0,
            color: general.border,
        },
        ..Default::default()
    }
}

pub fn error_tooltip(theme: &Theme) -> Style {
    let general = theme.colors().general;
    let text = theme.colors().text;

    Style {
        background: Some(Background::Color(general.background)),
        border: Border {
            radius: 4.0.into(),
            width: 1.0,
            color: text.error,
        },
        ..Default::default()
    }
}

pub fn transparent_overlay(theme: &Theme) -> Style {
    let general = theme.colors().general;

    Style {
        //TODO: Blur background when possible?
        background: Some(Background::Color(Color {
            a: 0.7,
            ..general.background
        })),
        ..Default::default()
    }
}
