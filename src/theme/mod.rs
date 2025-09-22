use crate::color::Color;

mod vscode;

use serde::{Deserialize, Serialize};
pub use vscode::parse_vscode_theme;

#[derive(Debug, Clone)]
pub struct Theme {
    #[allow(dead_code)]
    pub name: String,
    pub style: Style,
    pub gutter_style: Style,
    pub status_line_style: StatusLineStyle,
    pub token_styles: Vec<TokenStyle>,
    pub selection_style: Option<Style>,
}

impl Theme {
    pub fn get_style(&self, scope: &str) -> Option<Style> {
        self.token_styles.iter().find_map(|token_style| {
            if token_style.scope.contains(&scope.to_string()) {
                Some(token_style.style.clone())
            } else {
                None
            }
        })
    }

    #[allow(dead_code)]
    pub fn get_selection_background(&self) -> Color {
        self.selection_style
            .as_ref()
            .and_then(|s| s.background)
            .unwrap_or(Color::Rgb {
                r: 255,
                g: 255,
                b: 255,
            })
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            style: Style {
                foreground: Some(Color::Rgb {
                    r: 255,
                    g: 255,
                    b: 255,
                }),
                background: Some(Color::Rgb { r: 0, g: 0, b: 0 }),
                bold: false,
                italic: false,
            },
            gutter_style: Style::default(),
            status_line_style: StatusLineStyle::default(),
            token_styles: vec![],
            selection_style: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenStyle {
    pub name: Option<String>,
    pub scope: Vec<String>,
    pub style: Style,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct StatusLineStyle {
    pub outer_style: Style,
    pub outer_chars: [char; 4],
    pub inner_style: Style,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Style {
    pub foreground: Option<Color>,
    pub background: Option<Color>,
    pub bold: bool,
    pub italic: bool,
}

impl Style {
    #[allow(dead_code)]
    pub fn fallback_background(&self, fallback_background: &Style) -> Style {
        let background = self
            .background
            .or(fallback_background.background)
            .or(Some(Color::Rgb { r: 0, g: 0, b: 0 }));
        self.with_background(background)
    }

    #[allow(dead_code)]
    pub fn with_background(&self, background: Option<Color>) -> Style {
        Style {
            background,
            ..self.clone()
        }
    }

    #[allow(dead_code)]
    pub fn inverted(&self) -> Style {
        Style {
            foreground: self.background,
            background: self.foreground,
            bold: self.bold,
            italic: self.italic,
        }
    }
}
