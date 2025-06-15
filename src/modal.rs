use iced::Task;

use crate::widget::Element;

pub mod command_palette;

#[derive(Debug)]
pub enum Modal {
    CommandPalette(command_palette::State),
}

#[derive(Debug, Clone)]
pub enum ModalMessage {
    CommandPalette(command_palette::Message),
    Cancel,
}

pub enum Event {
    CloseModal,
}

impl Modal {
    pub fn update(&mut self, message: &ModalMessage) -> (Task<ModalMessage>, Option<Event>) {
        match (self, message) {
            (_, ModalMessage::Cancel) => (Task::none(), Some(Event::CloseModal)),

            (Modal::CommandPalette(state), ModalMessage::CommandPalette(msg)) => {
                let (task, event) = state.update(msg);
                (task.map(ModalMessage::CommandPalette), event)
            }

            _ => (Task::none(), None),
        }
    }

    pub fn view(&self) -> Element<'_, ModalMessage> {
        match self {
            Modal::CommandPalette(state) => state.view().map(ModalMessage::CommandPalette),
        }
    }
}
