use iced::{Element, Alignment, Length};
use iced::widget::{text_input, Container, TextInput, Button, Column, Text};

use super::{Modal, ModalMessage};

#[derive(Debug, Clone)]
pub struct CommandPalette {
    input_value: String,
    input_state: text_input::State<iced::widget::text::Renderer>,
}

impl CommandPalette {
    pub fn new() -> Self {
        Self {
            input_value: String::new(),
            input_state: text_input::State::new(),
        }
    }
}

impl Modal for CommandPalette {
    fn view(&self) -> Element<'_, ModalMessage> {
        let input = TextInput::new("Type a command...", &self.input_value)
            .on_input(|s| ModalMessage::Custom(s))
            .state(&self.input_state);

            let content = Column::new()
            .spacing(16)
        .align_x(Alignment::Center)
    .push(Text::new("Command Palette"))
    .push(input)
    .push(Button::new("Close").on_press(ModalMessage::Close));

    Container::new(content)
        .width(Length::Fixed(400.0))
    .padding(20)
    .align_x(Alignment::Center)
.align_y(Alignment::Center)
.into()
    }
}
