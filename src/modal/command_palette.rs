use iced::{
    Length, alignment,
    widget::{button, column, container, text, vertical_space},
};

use super::Message;
use crate::widget::Element;

pub fn view() -> Element<'static, Message> {
    container(column![])
        .max_width(400)
        .width(Length::Shrink)
        .padding(25)
        .into()
}
