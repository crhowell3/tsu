use iced::{
    Length, Task,
    widget::{Button, Scrollable, column, container, text, text_input},
};

use crate::{theme, widget::Element};

#[derive(Debug, Default)]
pub struct State {
    pub input_value: String,
    pub commands: Vec<String>,
    pub filtered: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    InputChanged(String),
    ExecuteCommand(String),
}

impl State {
    pub fn new(commands: Vec<String>) -> Self {
        Self {
            commands: commands.clone(),
            filtered: commands,
            ..Default::default()
        }
    }

    pub fn update(&mut self, message: &Message) -> (Task<Message>, Option<super::Event>) {
        match message {
            Message::InputChanged(input) => {
                self.input_value = input.clone();
                self.filtered = self
                    .commands
                    .iter()
                    .filter(|cmd| cmd.to_lowercase().contains(&input.to_lowercase()))
                    .cloned()
                    .collect();
                (Task::none(), None)
            }
            Message::ExecuteCommand(cmd) => (Task::none(), Some(super::Event::CloseModal)),
        }
    }

    pub fn view<'a>(&self) -> Element<'a, Message> {
        let input = text_input("Start typing...", &self.input_value)
            .on_input(Message::InputChanged)
            .padding(10)
            .size(20)
            .width(Length::Fill);

        let command_buttons: Vec<Element<'_, Message>> = self
            .filtered
            .iter()
            .map(|cmd| {
                Button::new(text(cmd))
                    .on_press(Message::ExecuteCommand(cmd.clone()))
                    .padding(5)
                    .into()
            })
            .collect();

        // let command_list: Scrollable<'a, Message> =
        //     Scrollable::new(column(command_buttons).spacing(5)).height(Length::Fixed(200.0));

        let content = column![input, /*command_list*/]
            .padding(20)
            .spacing(10)
            .width(Length::Fixed(400.0));

        container(content)
            .padding(20)
            .width(Length::Shrink)
            .height(Length::Shrink)
            .style(theme::container::tooltip)
            .into()
    }
}
