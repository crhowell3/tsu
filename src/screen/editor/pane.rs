use data::Config;
use iced::widget::{button, center, container, pane_grid, row, text};

use super::sidebar;
use crate::widget::{TextEditor, tooltip};
use crate::{Theme, icon, theme, widget};

#[derive(Debug, Clone)]
pub enum Message {
    PaneClicked(pane_grid::Pane),
    PaneResized(pane_grid::ResizeEvent),
    PaneDragged(pane_grid::DragEvent),
    TextEditor(pane_grid::Pane),
    ClosePane,
    SplitPane(pane_grid::Axis),
    MaximizePane,
    ToggleShowUserList,
    ToggleShowTopic,
    Popout,
    Merge,
    ScrollToBottom,
    MarkAsRead,
}

#[derive(Clone, Debug)]
pub struct Pane<'a> {
    pub text_editor: TextEditor<'a>,
    title_bar: TitleBar,
}

#[derive(Debug, Clone, Default)]
pub struct TitleBar {}

impl<'a> Pane<'a> {
    pub fn new(text_editor: TextEditor<'a>) -> Self {
        Self {
            text_editor,
            title_bar: TitleBar::default(),
        }
    }

    pub fn view<'a>(
        &'a self,
        id: pane_grid::Pane,
        panes: usize,
        is_focused: bool,
        maximized: bool,
        sidebar: &'a sidebar::Sidebar,
        config: &'a Config,
        theme: &'a Theme,
        settings: Option<&'a text_editor::Settings>,
        is_popout: bool,
    ) -> widget::Content<'a, Message> {
        let title_bar_text = "File";

        let title_bar = self.title_bar.view(
            title_bar_text,
            id,
            panes,
            is_focused,
            maximized,
            settings,
            config.tooltips,
            is_popout,
            config,
        );

        let status = row![
            text(if let Some(path) = &self.file {
                let path = path.display().to_string();

                if path.len() > 60 {
                    format!("...{}", &path[path.len() - 40..])
                } else {
                    path
                }
            } else {
                String::from("New file")
            }),
            horizontal_space(),
            text({
                let (line, column) = self.content.cursor_position();

                format!("{}:{}", line + 1, column + 1)
            })
        ]
        .spacing(10);

        let editor_pane = container(
            column![
                text_editor(&self.content)
                    .height(Length::Fill)
                    .wrapping(if self.word_wrap {
                        text::Wrapping::Word
                    } else {
                        text::Wrapping::None
                    }),
                status,
            ]
            .spacing(10)
            .padding(10),
        );

        let content = self
            .buffer
            .view(settings, config, theme, is_focused, sidebar)
            .map(move |msg| Message::Buffer(id, msg));

        widget::Content::new(content)
            .style(move |theme| theme::container::buffer(theme, is_focused))
            .title_bar(title_bar.style(theme::container::buffer_title_bar))
    }
}

impl TitleBar {
    fn view<'a>(
        &'a self,
        value: String,
        _id: pane_grid::Pane,
        panes: usize,
        _is_focused: bool,
        maximized: bool,
        settings: Option<&'a buffer::Settings>,
        show_tooltips: bool,
        is_popout: bool,
        config: &'a Config,
    ) -> widget::TitleBar<'a, Message> {
        // Pane controls.
        let mut controls = row![].spacing(2);

        // If we have more than one pane open, show maximize button.
        if panes > 1 {
            let maximize_button = button(center(if maximized {
                icon::restore()
            } else {
                icon::maximize()
            }))
            .padding(5)
            .width(22)
            .height(22)
            .on_press(Message::MaximizePane)
            .style(move |theme, status| theme::button::secondary(theme, status, maximized));

            let maximize_button_with_tooltip = tooltip(
                maximize_button,
                show_tooltips.then_some(if maximized { "Restore" } else { "Maximize" }),
                tooltip::Position::Bottom,
            );

            controls = controls.push(maximize_button_with_tooltip);
        }

        // Button to merge popout back in to main window
        if is_popout {
            let merge_button = button(center(icon::popout()))
                .padding(5)
                .width(22)
                .height(22)
                .on_press(Message::Merge)
                .style(|theme, status| theme::button::secondary(theme, status, true));

            let close_button_with_tooltip = tooltip(
                merge_button,
                show_tooltips.then_some("Merge"),
                tooltip::Position::Bottom,
            );

            controls = controls.push(close_button_with_tooltip);
        }
        // Allow pane to be pop'd out if we have >1 pane on main window
        else if panes > 1 {
            let popout_button = button(center(icon::popout()))
                .padding(5)
                .width(22)
                .height(22)
                .on_press(Message::Popout)
                .style(|theme, status| theme::button::secondary(theme, status, false));

            let close_button_with_tooltip = tooltip(
                popout_button,
                show_tooltips.then_some("Pop Out"),
                tooltip::Position::Bottom,
            );

            controls = controls.push(close_button_with_tooltip);
        }

        let title = container(text(value).shaping(text::Shaping::Advanced))
            .height(22)
            .padding([0, 10])
            .align_y(iced::alignment::Vertical::Center);

        widget::TitleBar::new(title)
            .controls(pane_grid::Controls::new(controls))
            .padding(6)
    }
}

impl From<Pane> for data::Pane {
    fn from(pane: Pane) -> Self {
        let _ = pane;
        data::Pane::Empty
    }
}
