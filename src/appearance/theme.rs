pub use data::appearance::theme::{
    Button, Buttons, Colors, General, Text, color_to_hex, hex_to_color,
};

pub mod container;

pub const TEXT_SIZE: f32 = 13.0;
pub const ICON_SIZE: f32 = 12.0;

#[derive(Debug, Clone)]
pub enum Theme {
    Selected(),
    Preview {
        selected: data::Theme,
        preview: data::Theme,
    },
}

impl Theme {
    pub fn preview(&self, theme: data::Theme) -> Self {
        match self {
            Theme::Selected(selected) | Theme::Preview { selected, .. } => Self::Preview {
                selected: selected.clone(),
                preview: theme,
            },
        }
    }

    pub fn selected(&self) -> Self {
        match self {
            Theme::Selected(selected) | Theme::Preview { selected, .. } => {
                Self::Selected(selected.clone())
            }
        }
    }

    pub fn colors(&self) -> &Colors {
        match self {
            Theme::Selected(selected) => &selected.colors,
            Theme::Preview { preview, .. } => &preview.colors,
        }
    }
}

impl From<data::Theme> for Theme {
    fn from(theme: data::Theme) -> Self {
        Theme::Selected(theme)
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::from(data::Theme::default())
    }
}

impl iced::theme::Theme for Theme {
    fn base(&self) -> iced::theme::Style {
        iced::theme::Style {
            background_color: self.colors().general.background,
            text_color: self.colors().text.primary,
        }
    }

    fn palette(&self) -> Option<iced::theme::Palette> {
        None
    }
}

impl combo_box::Catalog for Theme {}
