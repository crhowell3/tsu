use crossterm::style::Color;

use serde::Deserialize;
use serde_json::{Map, Value};

#[derive(Debug, Default)]
pub struct Style {
    foreground: Option<Color>,
    background: Option<Color>,
    bold: bool,
    italic: bool,
}

#[derive(Debug)]
struct TokenStyle {
    name: Option<String>,
    scope: Vec<String>,
    style: Style,
}

#[derive(Debug)]
pub struct Theme {
    name: String,
    style: Style,
    token_styles: Vec<TokenStyle>,
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
enum VSCodeScope {
    Single(String),
    Multiple(Vec<String>),
}

impl From<VSCodeScope> for Vec<String> {
    fn from(scope: VSCodeScope) -> Self {
        match scope {
            VSCodeScope::Single(s) => vec![s],
            VSCodeScope::Multiple(v) => v,
        }
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct VSCodeTokenColor {
    name: Option<String>,
    scope: Option<VSCodeScope>,
    settings: Map<String, Value>,
}

impl TryFrom<VSCodeTokenColor> for TokenStyle {
    type Error = anyhow::Error;

    fn try_from(value: VSCodeTokenColor) -> Result<Self, Self::Error> {
        let mut style = Style::default();

        if let Some(foreground) = value.settings.get("foreground") {
            style.foreground = Some(
                parse_rgb(foreground.as_str().expect("foreground is string"))
                    .expect("converting string to rgb works"),
            );
        }

        if let Some(background) = value.settings.get("background") {
            style.background = Some(
                parse_rgb(background.as_str().expect("soreground is string"))
                    .expect("converting string to rgb works"),
            );
        }

        if let Some(font_style) = value.settings.get("fontStyle") {
            style.bold = font_style
                .as_str()
                .expect("fontStyle is string")
                .contains("bold");
            style.italic = font_style
                .as_str()
                .expect("fontStyle is string")
                .contains("italic");
        }

        let Some(scope) = value.scope else {
            return Err(anyhow::anyhow!("TokenColor has no scope"));
        };

        Ok(Self {
            name: value.name,
            scope: scope.into(),
            style,
        })
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct VSCodeTheme {
    name: Option<String>,
    #[serde(rename = "type")]
    obj_type: Option<String>,
    colors: Map<String, Value>,
    token_colors: Vec<VSCodeTokenColor>,
}

fn parse_rgb(s: &str) -> anyhow::Result<Color> {
    if !s.starts_with('#') {
        anyhow::bail!("Invalid hex string: {}", s);
    }

    let hex = s.trim_start_matches('#');
    let len = hex.len();

    if len != 6 && len != 8 {
        anyhow::bail!(
            "Hex string must be in the format #RRGGBB or #RRGGBBAA instead of {}",
            s
        );
    }

    let r = u8::from_str_radix(&hex[0..2], 16)?;
    let g = u8::from_str_radix(&hex[2..4], 16)?;
    let b = u8::from_str_radix(&hex[4..6], 16)?;

    Ok(Color::Rgb { r, g, b })
}

pub fn parse_vscode_theme(file: &str) -> anyhow::Result<Theme> {
    let contents = std::fs::read_to_string(file)?;
    let vscode_theme: VSCodeTheme = serde_json::from_str(&contents)?;

    let (token_colors_with_scope, token_colors_without_scope): (
        Vec<VSCodeTokenColor>,
        Vec<VSCodeTokenColor>,
    ) = vscode_theme
        .token_colors
        .into_iter()
        .partition(|tc| tc.scope.is_some());

    let token_styles = token_colors_with_scope
        .into_iter()
        .map(|tc| tc.try_into())
        .collect::<Result<Vec<TokenStyle>, _>>()?;

    let foreground_token_color = token_colors_without_scope
        .iter()
        .find(|tc| tc.settings.contains_key("foreground"));
    let background_token_color = token_colors_without_scope
        .iter()
        .find(|tc| tc.settings.contains_key("background"));

    let foreground = match foreground_token_color {
        Some(tc) => tc.settings.get("foreground"),
        None => vscode_theme.colors.get("editor.foreground"),
    };
    let background = match background_token_color {
        Some(tc) => tc.settings.get("background"),
        None => vscode_theme.colors.get("editor.background"),
    };

    Ok(Theme {
        name: vscode_theme.name.unwrap_or_default(),
        style: Style {
            foreground: Some(parse_rgb(
                foreground
                    .expect("foreground color exists")
                    .as_str()
                    .expect("foreground color is string"),
            )?),
            background: Some(parse_rgb(
                background
                    .expect("background color exists")
                    .as_str()
                    .expect("background color is string"),
            )?),
            bold: false,
            italic: false,
        },
        token_styles,
    })
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse_vscode() {
        let theme = parse_vscode_theme("./src/fixtures/tokyo-night-storm.json").unwrap();
        println!("{:?}", theme);
    }
}
