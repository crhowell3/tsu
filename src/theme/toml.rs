use crate::color::parse_rgb;

use super::{StatusLineStyle, Style, Theme, TokenStyle};

pub fn parse_toml_theme(file: &str) -> anyhow::Result<Theme> {
    let contents = std::fs::read_to_string(file)?;
    let toml_theme = contents.parse::<toml::Table>().unwrap();

    let foreground = &toml_theme["fg1"];
    let background = &toml_theme["bg1"];

    let token_styles;

    let gutter_style;

    let selection_style;

    let status_line_style = StatusLineStyle {
        outer_style: Style {
            foreground: Some(parse_rgb(
                toml_theme["ui.statusline"]
                    .as_str()
                    .expect("foreground color is string"),
            )?),
            background: Some(parse_rgb(
                toml_theme["ui.statusline.normal"]
                    .as_str()
                    .expect("background color is string"),
            )?),
            bold: true,
            ..Default::default()
        },
        outer_chars: [' ', '', '', ' '],
        inner_style: Style {
            foreground: Some(parse_rgb(
                toml_theme["fg1"]
                    .as_str()
                    .expect("foreground color is string"),
            )?),
            background: Some(parse_rgb(
                toml_theme["bg1"]
                    .as_str()
                    .expect("background color is string"),
            )?),
            ..Default::default()
        },
    };

    Ok(Theme {
        name: "".to_owned(),
        style: Style {
            foreground: Some(parse_rgb(
                foreground.as_str().expect("foreground color is string"),
            )?),
            background: Some(parse_rgb(
                background.as_str().expect("background color is string"),
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
