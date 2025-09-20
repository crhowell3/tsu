use crossterm::style::Color;

pub struct Style {
    foreground: Color,
    background: Color,
    bold: bool,
    italic: bool,
    underline: bool,
}

pub struct Theme {
    name: String,
    style: Style,
}

pub fn parse_vscode_theme(file: &str) -> Theme {}
