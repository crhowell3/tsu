use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Color {
    Rgb { r: u8, g: u8, b: u8 },
    Rgba { r: u8, g: u8, b: u8, a: u8 },
}

impl Default for Color {
    fn default() -> Self {
        Color::Rgb { r: 0, g: 0, b: 0 }
    }
}

impl From<Color> for crossterm::style::Color {
    fn from(color: Color) -> Self {
        match color {
            Color::Rgb { r, g, b } => crossterm::style::Color::Rgb { r, g, b },
            Color::Rgba { r, g, b, a: _ } => crossterm::style::Color::Rgb { r, g, b },
        }
    }
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Color::Rgb { r, g, b } => write!(f, "#{:02x}{:02x}{:02x}", r, g, b),
            Color::Rgba { r, g, b, a } => write!(f, "#{:02x}{:02x}{:02x}{:02x}", r, g, b, a),
        }
    }
}

pub fn parse_rgb(s: &str) -> anyhow::Result<Color> {
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
