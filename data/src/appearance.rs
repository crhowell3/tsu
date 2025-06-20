pub use theme::Theme;

pub mod theme;

#[derive(Debug, Clone)]
pub struct Appearance {
    pub selected: Selected,
    pub all: Vec<Theme>,
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            selected: Selected::default(),
            all: vec![Theme::default()],
        }
    }
}

#[derive(Debug, Clone)]
pub enum Selected {
    Static(Theme),
    Dynamic { light: Theme, dark: Theme },
}

impl Default for Selected {
    fn default() -> Self {
        Self::Static(Theme::default())
    }
}

impl Selected {
    #[must_use]
    pub fn is_dynamic(&self) -> bool {
        match self {
            Selected::Static(_) => false,
            Selected::Dynamic { .. } => true,
        }
    }

    #[must_use]
    pub fn dynamic(light: Theme, dark: Theme) -> Selected {
        Selected::Dynamic { light, dark }
    }

    #[must_use]
    pub fn specific(theme: Theme) -> Selected {
        Selected::Static(theme)
    }
}
