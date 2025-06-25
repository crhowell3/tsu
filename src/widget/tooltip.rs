pub use iced::widget::tooltip::Position;
use iced::widget::{container, text};

use super::Element;
use crate::theme;

pub fn tooltip<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
    tooltip: Option<&'a str>,
    position: Position,
) -> Element<'a, Message> {
    match tooltip {
        Some(tooltip) => iced::widget::tooltip(
            content,
            container(text(tooltip).style(theme::text::secondary))
                .style(theme::container::tooltip)
                .padding(8),
            position,
        )
        .into(),
        None => content.into(),
    }
}
