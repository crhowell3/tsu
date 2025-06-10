pub mod command_palette;

use iced::Element;

#[derive(Debug, Clone)]
pub enum ModalMessage {
    Close,
    Custom(String),
}

pub trait Modal {
    fn view(&self) -> Element<'_, ModalMessage>;
}
