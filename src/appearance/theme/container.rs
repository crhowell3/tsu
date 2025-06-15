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

pub fn general(theme: &Theme) -> Style {
    Style {
        background: Some(Background::Color(theme.colors().general.background)),
        text_color: Some(theme.colors().text.primary),
        ..Default::default()
    }
}
