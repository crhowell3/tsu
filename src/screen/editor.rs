use std::path::PathBuf;
use std::{convert, slice};

use chrono::{DateTime, Utc};
use data::environment::{RELEASE_WEBSITE};
use data::{Config, command, config, environment};
use iced::widget::pane_grid::{self, PaneGrid};
use iced::widget::{column, container, horizontal_space, row, text, text_editor, Space};
use iced::window::get_position;
use iced::{Length, Task, Vector, clipboard};
use iced::keyboard;

use self::command_palette::CommandPalette;
use self::sidebar::Sidebar;
use self::theme_editor::ThemeEditor;
use crate::widget::{Column, Element, Row, anchored_overlay, shortcut};
use crate::window::Window;
use crate::{Theme, event, theme, window};

mod command_palette;
pub mod pane;
pub mod sidebar;
mod theme_editor;

pub struct Editor {
    panes: Panes,
    focus: Focus,
    side_menu: Sidebar,
    command_palette: Option<CommandPalette>,
    theme_editor: Option<ThemeEditor>,
}

#[derive(Debug)]
pub enum Message {
    Pane(window::Id, pane::Message),
    Sidebar(sidebar::Message),
    Task(command_palette::Message),
    Shortcut(shortcut::Command),
    CloseContextMenu(window::Id, bool),
    ThemeEditor(theme_editor::Message),
    NewWindow(window::Id, Pane),
}

#[derive(Debug)]
pub enum Event {
    ConfigReloaded(Result<Config, config::Error>),
    ReloadThemes,
    Exit,
}

impl Editor {
    pub fn view_window<'a>(
        &'a self,
        window: window::Id,
        config: &'a Config,
        theme: &'a Theme
    ) -> Element<'a, Message> {
        if let Some(state) = self.panes.popout.get(&window) {
            let content = container(PaneGrid::new(state, |id, pane, _maximized| {
                let is_focused = self.focus == Focus {
                    window, pane: id};
                    let editor = pane.editor;

                    pane.view(
                        id, 1, is_focused, false, config, theme, settings, window != self.main_window(),
                    )
                })
                .spacing(4)
                .on_click(pane::Message::PaneClicked),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(8);

        return Element::new(content).map(move |message| Message::Pane(window, message));
        } else if let Some(editor) = self.theme_editor.as_ref() {
            if editor.window == window {
                return editor.view(theme).map(Message::ThemeEditor);
            }
        }

        column![].into()
    }
    pub fn view<'a>(&'a self, config: &'a Config, theme: &'a Theme) -> Element<'a, Message> {
        let pane_grid: Element<_> =
        PaneGrid::new(&self.panes.main, |id, pane, maximized| {
            let is_focused = self.focus == Focus {
                window: self.main_window(),
                pane: id,
            };
            let panes = self.panes.main.panes.len();
            let editor = pane.editor;

            pane.view(id, panes, is_focused, maximized, config, theme, false)
        })
        .on_click(pane::Message::PaneClicked)
        .on_resize(6, pane::Message::PaneResized)
        .on_drag(pane::Message::PaneDragged)
        .spacing(4)
        .into();

        let pane_grid = container(pane_grid.map(move |message| {
            Message::Pane(self.main_window(), message)
        }))
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(8);

        let side_menu = self.side_menu.view(config).map(|e| e.map(Message::Sidebar));
        let content = match config.sidebar.position {
            data::config::sidebar::Position::Left => {
                vec![
                    side_menu.unwrap_or_else(|| row![].into()),
                    pane_grid.into(),
                ]
            }
        };

        let base: Element<Message> = if config.sidebar.position.is_horizontal() {
            Column::with_children(content)
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        } else {
            Row::with_children(content)
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        };

        let base = if let Some(command_palette) = self.command_palette.as_ref() {
            let background = anchored_overlay(
                base,
                container(Space::new(Length::Fill, Length::Fill))
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .style(theme::container::transparent_overlay),
                anchored_overlay::Anchor::BelowTopCentered,
                0.0,
            );

            anchored_overlay(
                background,
                command_palette
                    .view(self.focus, config, self.main_window())
                    .map(Message::Task),
                anchored_overlay::Anchor::BelowTopCentered,
                10.0,
            )
        } else {
            column![column![base]].into()
        };

        shortcut(base, config.keyboard.shortcuts(), Message::Shortcut)
    }

    pub fn handle_event(&mut Self, window: window::Id, event: event::Event, config: &Config, theme: &mut Theme) -> Task<Message> {
        use event::Event::*;

        match event {
            Escape => {
                if self.command_palette.is_some() && window == self.main_window() {
                    self.toggle_command_palette(
                        config, theme,
                    )
                } else {
                    context_menu::close(convert::identity).map(
                        move |any_closed| {
                            Message::CloseContextMenu(window, any_closed)
                        },
                    )
                }
            }
        }
    }

    pub fn toggle_command_palette(
        &mut self,
        config: &Config,
        theme: &mut Theme,
    ) -> Task<Message> {
        if self.command_palette.is_some() {
            *theme = theme.selected();

            self.close_command_palette();
            let Focus { window } = self.focus;
            Task::none()
        } else {
            self.open_command_palette(config);
            Task::none()
        }
    }

    fn open_command_palette(
        &mut self, config: &Config
    ) {
        self.command_palette = Some(CommandPalette::new(
            config, self.focus, self.main_window()
        ));
    }

    fn close_command_palette(&mut self) {
        self.command_palette = None;
    }

    fn main_window(&self) -> window::Id {
        self.panes.main_window
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Focus {
    pub window: window::Id,
    pub pane: pane_grid::Pane,
}

impl<'a> From<&'a Editor> for data::Editor {
    fn from(editor: &'a Editor) -> Self {
        use pane_grid::Node;

        fn from_layout(
            panes: &pane_grid::State<Pane>,
            node: pane_grid::Node,
        ) -> data::Pane {
            match node {
                Node::Split {
                    axis, ratio, a, b, ..
                } => data::Pane::Split {
                    axis: match axis {
                        pane_grid::Axis::Horizontal => {
                            data::pane::Axis::Horizontal
                        }
                        pane_grid::Axis::Vertical => data::pane::Axis::Vertical,
                    },
                    ratio,
                    a: Box::new(from_layout(panes, *a)),
                    b: Box::new(from_layout(panes, *b)),
                },
                Node::Pane(pane) => panes
                    .get(pane)
                    .cloned()
                    .map_or(data::Pane::Empty, data::Pane::from),
            }
        }

        let layout = editor.panes.main.layout().clone();
        let focus = editor.focus;

        data::Editor {
            pane: from_layout(&editor.panes.main, layout),
            popout_panes: editor
                .panes
                .popout
                .values()
                .map(|state| from_layout(state, state.layout().clone()))
                .collect(),
        }
    }
}

#[derive(Clone)]
pub struct Panes {
    main_window: window::Id,
    main: pane_grid::State<Pane>,
    popout: HashMap<window::Id, pane_grid::State<Pane>>,
}

impl Panes {
    fn len(&self) -> usize {
        self.main.panes.len() + self.popout.len()
    }

    fn get(&self, window: window::Id, pane: pane_grid::Pane) -> Option<&Pane> {
        if self.main_window == window {
            self.main.get(pane)
        } else {
            self.popout.get(&window).and_then(|panes| panes.get(pane))
        }
    }

    fn get_mut(
        &mut self,
        window: window::Id,
        pane: pane_grid::Pane,
    ) -> Option<&mut Pane> {
        if self.main_window == window {
            self.main.get_mut(pane)
        } else {
            self.popout
                .get_mut(&window)
                .and_then(|panes| panes.get_mut(pane))
        }
    }

    fn iter(
        &self,
    ) -> impl Iterator<Item = (window::Id, pane_grid::Pane, &Pane)> {
        self.main
            .iter()
            .map(move |(pane, state)| (self.main_window, *pane, state))
            .chain(self.popout.iter().flat_map(|(window_id, panes)| {
                panes.iter().map(|(pane, state)| (*window_id, *pane, state))
            }))
    }

    fn iter_mut(
        &mut self,
    ) -> impl Iterator<Item = (window::Id, pane_grid::Pane, &mut Pane)> {
        let main_window = self.main_window;

        self.main
            .iter_mut()
            .map(move |(pane, state)| (main_window, *pane, state))
            .chain(self.popout.iter_mut().flat_map(|(window_id, panes)| {
                panes
                    .iter_mut()
                    .map(|(pane, state)| (*window_id, *pane, state))
            }))
    }

    fn resources(&self) -> impl Iterator<Item = data::history::Resource> + '_ {
        self.main.panes.values().filter_map(Pane::resource).chain(
            self.popout.values().flat_map(|state| {
                state.panes.values().filter_map(Pane::resource)
            }),
        )
    }
}
