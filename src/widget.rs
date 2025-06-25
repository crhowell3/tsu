#![allow(dead_code)]

pub use self::anchored_overlay::anchored_overlay;
pub use self::combo_box::combo_box;
pub use self::context_menu::context_menu;
pub use self::decorate::decorate;
pub use self::double_pass::double_pass;
pub use self::key_press::key_press;
pub use self::shortcut::shortcut;
use crate::Theme;

pub mod anchored_overlay;
pub mod color_picker;
pub mod combo_box;
pub mod context_menu;
pub mod decorate;
pub mod double_pass;
pub mod key_press;
pub mod selectable_rich_text;
pub mod selectable_text;
pub mod shortcut;
pub mod tooltip;

pub type Renderer = iced::Renderer;
pub type Element<'a, Message> = iced::Element<'a, Message, Theme, Renderer>;
pub type Content<'a, Message> = iced::widget::pane_grid::Content<'a, Message, Theme, Renderer>;
pub type TitleBar<'a, Message> = iced::widget::pane_grid::TitleBar<'a, Message, Theme, Renderer>;
pub type Column<'a, Message> = iced::widget::Column<'a, Message, Theme, Renderer>;
pub type Row<'a, Message> = iced::widget::Row<'a, Message, Theme, Renderer>;
pub type Text<'a> = iced::widget::Text<'a, Theme, Renderer>;
pub type Container<'a, Message> = iced::widget::Container<'a, Message, Theme, Renderer>;
pub type Button<'a, Message> = iced::widget::Button<'a, Message, Theme>;
pub type Scrollable<'a, Message> = iced::widget::Scrollable<'a, Message, Theme, Renderer>;

pub mod button {
    use super::Element;
    use crate::appearance::theme;

    pub fn transparent_button<'a, Message>(
        content: impl Into<Element<'a, Message>>,
        message: Message,
    ) -> Element<'a, Message>
    where
        Message: Clone + 'a,
    {
        iced::widget::button(content)
            .padding(0)
            .style(theme::button::bare)
            .on_press(message)
            .into()
    }
}
