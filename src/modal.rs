use iced::Task;

use crate::widget::Element;
use crate::window;

pub mod command_palette;

#[derive(Debug)]
pub enum Modal {
    CommandPalette(command_palette::State),
}

#[derive(Debug, Clone)]
pub enum Message {
    CommandPalette(command_palette::Message),
    Cancel,
}

pub enum Event {
    CloseModal,
}

impl Modal {
    pub fn window_id(&self) -> Option<window::Id> {
        match self {
            Modal::CommandPalette(..) => None,
        }
    }

    pub fn update(&mut self, message: &Message) -> (Task<Message>, Option<Event>) {
        match (self, message) {
            (_, Message::Cancel) => (Task::none(), Some(Event::CloseModal)),

            (Modal::CommandPalette(state), Message::CommandPalette(msg)) => {
                let (task, event) = state.update(msg);
                (task.map(Message::CommandPalette), event)
            }
        }
    }

    pub fn view(&self) -> Element<Message> {
        match self {
            Modal::CommandPalette(state) => state.view().map(Message::CommandPalette),
        }
    }
}
