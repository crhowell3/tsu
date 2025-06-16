use iced::window::{Id, Position, Settings, close, gain_focus, get_latest, open};

use iced::{Point, Size, Subscription, Task};

#[derive(Debug, Clone, Copy)]
pub struct Window {
    pub id: Id,
    pub position: Option<Point>,
    pub size: Size,
    pub focused: bool,
}

impl Window {}
