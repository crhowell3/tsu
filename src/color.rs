use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
/// The internal representation of a hexadecimal RGB/RGBA color
///
/// This internal representation serves as an intermediate representation of "color". It can be
/// converted to and from `crossterm::style::Color::Rgb`, although it is lossy because crossterm
/// does not have an Rgba analogue
pub enum Color {
    /// A standard hexadecimal color representation parameterized as red, green, and blue
    Rgb { r: u8, g: u8, b: u8 },
    /// A hexadecimal color representation parameterized as red, green, blue, and alpha for
    /// transparency
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
            Color::Rgb { r, g, b } | Color::Rgba { r, g, b, a: _ } => {
                crossterm::style::Color::Rgb { r, g, b }
            }
        }
    }
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Color::Rgb { r, g, b } => write!(f, "#{r:02x}{g:02x}{b:02x}"),
            Color::Rgba { r, g, b, a } => write!(f, "#{r:02x}{g:02x}{b:02x}{a:02x}"),
        }
    }
}

/// Attempt to parse a hexadecimal string into a `Color`
///
/// # Arguments
/// - `s`: A hexadecimal string representation of a color, i.e., "#A3D1FD"
///
/// # Returns
/// - If successfully parsed, a `Color::Rgb` or `Color::Rgba`
///
/// # Errors
/// Can return a `ParseIntError` if it fails to convert any of the color channel strings into a
/// base 16 number
pub fn parse_rgb(s: &str) -> anyhow::Result<Color> {
    if !s.starts_with('#') {
        anyhow::bail!("Invalid hex string: {s}");
    }

    let hex = s.trim_start_matches('#');
    let len = hex.len();

    if len != 6 && len != 8 {
        anyhow::bail!("Hex string must be in the format #RRGGBB or #RRGGBBAA instead of {s}");
    }

    let r = u8::from_str_radix(&hex[0..2], 16)?;
    let g = u8::from_str_radix(&hex[2..4], 16)?;
    let b = u8::from_str_radix(&hex[4..6], 16)?;

    Ok(Color::Rgb { r, g, b })
}

pub fn blend_color(foreground: Color, background: Color) -> Color {
    match (foreground, background) {
        (
            Color::Rgba { r, g, b, a },
            Color::Rgb {
                r: background_r,
                g: background_g,
                b: background_b,
            },
        ) => {
            let alpha = a as f32 / 255.0;
            let inv_alpha = 1.0 - alpha;

            let r = (r as f32 * alpha + background_r as f32 * inv_alpha) as u8;
            let g = (g as f32 * alpha + background_g as f32 * inv_alpha) as u8;
            let b = (b as f32 * alpha + background_b as f32 * inv_alpha) as u8;

            Color::Rgb { r, g, b }
        }
        _ => foreground,
    }
}

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn test_parse_rgb_simple() {
        let hex_str = "#000000";
        assert_eq!(parse_rgb(hex_str).unwrap(), Color::Rgb { r: 0, g: 0, b: 0 });
    }

    #[test]
    fn test_parse_rgb_complex() {
        let hex_str = "#d35a87";
        assert_eq!(
            parse_rgb(hex_str).unwrap(),
            Color::Rgb {
                r: 0xd3,
                g: 0x5a,
                b: 0x87
            }
        );
    }
}
