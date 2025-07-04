use std::time::Duration;

use data::config::{self, Config, sidebar};
use iced::widget::{
    Column, Row, Scrollable, Space, button, column, container, horizontal_rule, horizontal_space,
    pane_grid, row, scrollable, text, vertical_rule, vertical_space,
};
use iced::{Alignment, Length, Task, padding};
use tokio::time;

use super::{Focus, Panes};
use crate::widget::{Element, Text, context_menu, double_pass};
use crate::{icon, theme, window};

const CONFIG_RELOAD_DELAY: Duration = Duration::from_secs(1);

#[derive(Debug, Clone)]
pub enum Message {
    Focus(window::Id, pane_grid::Pane),
    Close(window::Id, pane_grid::Pane),
}

#[derive(Debug, Clone)]
pub enum Event {
    Focus(window::Id, pane_grid::Pane),
    Close(window::Id, pane_grid::Pane),
}

#[derive(Clone)]
pub struct Sidebar {
    pub hidden: bool,
    reloading_config: bool,
}

impl Default for Sidebar {
    fn default() -> Self {
        Self::new()
    }
}

impl Sidebar {
    pub fn new() -> Self {
        Self {
            hidden: false,
            reloading_config: false,
        }
    }

    pub fn toggle_visibility(&mut self) {
        self.hidden = !self.hidden;
    }

    pub fn update(&mut self, message: Message) -> (Task<Message>, Option<Event>) {
        match message {
            Message::Focus(window, pane) => (Task::none(), Some(Event::Focus(window, pane))),
            Message::Close(window, pane) => (Task::none(), Some(Event::Close(window, pane))),
        }
    }

    pub fn view<'a>(
        &'a self,
        panes: &'a Panes,
        focus: Focus,
        config: &'a Config,
    ) -> Option<Element<'a, Message>> {
        if self.hidden {
            return None;
        }

        let content = |width| {
            let content = column![];
            container(content)
        };

        let padding = match config.sidebar.position {
            sidebar::Position::Left => padding::top(8).bottom(6).left(6),
            sidebar::Position::Right => padding::top(8).bottom(6).right(6),
        };

        let content =
            container(content(Length::Shrink).width(Length::Fill).padding(padding)).into();

        Some(content)
    }
}
