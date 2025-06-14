use iced::Task;

use crate::widget::Element;

pub mod command_palette;

#[derive(Debug)]
pub enum Modal {
    CommandPalette,
}

#[derive(Debug, Clone)]
pub enum Message {
    Cancel,
}

pub enum Event {
    CloseModal,
}

impl Modal {
    pub fn update(&mut self, message: &Message) -> (Task<Message>, Option<Event>) {
        match message {
            Message::Cancel => (Task::none(), Some(Event::CloseModal)),
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        match self {
            Modal::CommandPalette => command_palette::view(),
        }
    }
}
