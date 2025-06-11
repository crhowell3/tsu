use iced::{
    Length,
    widget::{column, container},
};

use super::Message;
use crate::widget::Element;
use crate::widget::Text;

pub fn view() -> Element<'static, Message> {
    container(column![Text::new("Command Palette")])
        .max_width(400)
        .width(Length::Shrink)
        .padding(25)
        .into()
}
