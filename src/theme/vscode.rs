use std::collections::HashMap;

use once_cell::sync::Lazy;

use crate::{
    color::{Color, parse_rgb},
    theme::StatusLineStyle,
};

use serde::Deserialize;
use serde_json::{Map, Value};

use super::{Style, Theme, TokenStyle};

static SYNTAX_HIGHLIGHTING_MAP: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
    let mut m = HashMap::new();

    m.insert("constant", "constant");
    m.insert("entity.name.type", "type");
    m.insert("support.type", "type");
    m.insert("entity.name.function.constructor", "constructor");
    m.insert("variable.other.enummember", "constructor");
    m.insert("entity.name.function", "function");
    m.insert("meta.function-call", "function");
    m.insert("entity.name.function.member", "function.method");
    m.insert("variable.function", "function.method");
    m.insert("entity.name.function.macro", "function.macro");
    m.insert("support.function.macro", "function.macro");
    m.insert("variable.other.member", "property");
    m.insert("variable.other.property", "property");
    m.insert("variable.parameter", "variable.parameter");
    m.insert("entity.name.label", "label");
    m.insert("comment", "comment");
    m.insert("punctuation.definition.comment", "comment");
    m.insert("punctuation.section.block", "punctuation.bracket");
    m.insert("punctuation.definition.brackets", "punctuation.bracket");
    m.insert("punctuation.separator", "punctuation.delimiter");
    m.insert("punctuation.accessor", "punctuation.delimiter");
    m.insert("keyword", "keyword");
    m.insert("keyword.control", "keyword");
    m.insert("support.type.primitive", "type.builtin");
    m.insert("keyword.type", "type.builtin");
    m.insert("variable.language", "variable.builtin");
    m.insert("support.variable", "variable.builtin");
    m.insert("string.quoted.double", "string");
    m.insert("string.quoted.single", "string");
    m.insert("constant.language", "constant.builtin");
    m.insert("constant.numeric", "constant.builtin");
    m.insert("constant.character", "constant.builtin");
    m.insert("constant.character.escape", "escape");
    m.insert("keyword.operator", "operator");
    m.insert("storage.modifier.attribute", "attribute");
    m.insert("meta.attribute", "attribute");

    m
});

fn translate_scope(theme_scope: String) -> String {
    SYNTAX_HIGHLIGHTING_MAP
        .get(&theme_scope.as_str())
        .map(|s| s.to_string())
        .unwrap_or(theme_scope)
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
            VSCodeScope::Single(s) => vec![translate_scope(s)],
            VSCodeScope::Multiple(v) => v.into_iter().map(translate_scope).collect(),
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
    #[allow(dead_code)]
    obj_type: Option<String>,
    colors: Map<String, Value>,
    token_colors: Vec<VSCodeTokenColor>,
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

    let gutter_style = Style {
        foreground: vscode_theme
            .colors
            .iter()
            .find(|(c, _)| **c == "editorLineNumber.foreground")
            .map(|(_, hex)| {
                parse_rgb(hex.as_str().expect("colors are formatted as a hex string")).unwrap()
            }),
        background: vscode_theme
            .colors
            .iter()
            .find(|(c, _)| **c == "editorLineNumber.background")
            .map(|(_, hex)| {
                parse_rgb(hex.as_str().expect("colors are formatted as a hex string")).unwrap()
            }),
        ..Default::default()
    };

    let status_line_style = StatusLineStyle {
        outer_style: Style {
            foreground: Some(Color::Rgb { r: 0, g: 0, b: 0 }),
            background: Some(Color::Rgb {
                r: 187,
                g: 154,
                b: 247,
            }),
            bold: true,
            ..Default::default()
        },
        outer_chars: [' ', '', '', ' '],
        inner_style: Style {
            foreground: Some(Color::Rgb {
                r: 255,
                g: 255,
                b: 255,
            }),
            background: Some(Color::Rgb {
                r: 65,
                g: 72,
                b: 104,
            }),
            ..Default::default()
        },
    };

    let selection_style = vscode_theme
        .colors
        .iter()
        .find(|(c, _)| **c == "editor.selectionBackground")
        .map(|(_, hex)| Style {
            background: Some(
                parse_rgb(hex.as_str().expect("colors are formatted as a hex string")).unwrap(),
            ),
            ..Default::default()
        });

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
        gutter_style,
        status_line_style,
        selection_style,
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

    #[test]
    fn test_parse_token_color_without_scope() {
        parse_vscode_theme("./src/fixtures/theme_colors_without_scope.json").unwrap();
    }

    #[test]
    fn test_theme_with_comments() {
        parse_vscode_theme("src/fixtures/theme_with_comments.json").unwrap();
    }
}
