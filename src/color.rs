#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_truncation)]
use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
/// The internal representation of a hexadecimal RGB/RGBA color
///
/// This internal representation serves as an intermediate representation of "color". It can be
/// converted to and from `crossterm::style::Color::Rgb`, although it is lossy because crossterm
/// does not have an Rgba analogue
pub enum Color {
    Reset,
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    Gray,
    LightRed,
    LightGreen,
    LightYellow,
    LightBlue,
    LightMagenta,
    LightCyan,
    LightGray,
    White,
    /// A standard hexadecimal color representation parameterized as red, green, and blue
    Rgb {
        r: u8,
        g: u8,
        b: u8,
    },
    /// A hexadecimal color representation parameterized as red, green, blue, and alpha for
    /// transparency
    Rgba {
        r: u8,
        g: u8,
        b: u8,
        a: u8,
    },
}

impl Default for Color {
    /// Generates a `Color` with default values
    ///
    /// # Returns
    /// - A default `Color`, which in this case is #000000, or black
    fn default() -> Self {
        Color::Rgb { r: 0, g: 0, b: 0 }
    }
}

impl From<Color> for crossterm::style::Color {
    /// Provides the logic for converting our custom `Color` to a `crossterm::style::Color`
    ///
    /// # Arguments
    /// - `color`: An instance of our custom `Color` struct
    ///
    /// # Returns
    /// - An instance of `crossterm::style::Color` derived from the custom `Color`
    fn from(color: Color) -> Self {
        match color {
            Color::Reset => crossterm::style::Color::Reset,
            Color::Black => crossterm::style::Color::Black,
            Color::Red | Color::LightRed => crossterm::style::Color::Red,
            Color::Green | Color::LightGreen => crossterm::style::Color::Green,
            Color::Yellow | Color::LightYellow => crossterm::style::Color::Yellow,
            Color::Blue | Color::LightBlue => crossterm::style::Color::Blue,
            Color::Magenta | Color::LightMagenta => crossterm::style::Color::Magenta,
            Color::Cyan | Color::LightCyan => crossterm::style::Color::Cyan,
            Color::Gray | Color::LightGray => crossterm::style::Color::Grey,
            Color::White => crossterm::style::Color::White,
            Color::Rgb { r, g, b } | Color::Rgba { r, g, b, a: _ } => {
                crossterm::style::Color::Rgb { r, g, b }
            }
        }
    }
}

impl fmt::Display for Color {
    /// Provides formatting for printing the contents of `Color` to the console
    ///
    /// # Arguments
    /// - `f`: Mutable reference to a `std::fmt::Formatter`
    ///
    /// # Returns
    /// - The result of the formatted string
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Color::Reset => write!(f, ""),
            Color::Black => write!(f, "{:?}", crossterm::style::Color::Black),
            Color::Red | Color::LightRed => write!(f, "{:?}", crossterm::style::Color::Red),
            Color::Green | Color::LightGreen => write!(f, "{:?}", crossterm::style::Color::Green),
            Color::Yellow | Color::LightYellow => {
                write!(f, "{:?}", crossterm::style::Color::Yellow)
            }
            Color::Blue | Color::LightBlue => write!(f, "{:?}", crossterm::style::Color::Blue),
            Color::Magenta | Color::LightMagenta => {
                write!(f, "{:?}", crossterm::style::Color::Magenta)
            }
            Color::Cyan | Color::LightCyan => write!(f, "{:?}", crossterm::style::Color::Cyan),
            Color::Gray | Color::LightGray => write!(f, "{:?}", crossterm::style::Color::Grey),
            Color::White => write!(f, "{:?}", crossterm::style::Color::White),
            Color::Rgb { r, g, b } => write!(f, "#{r:02x}{g:02x}{b:02x}"),
            Color::Rgba { r, g, b, a } => write!(f, "#{r:02x}{g:02x}{b:02x}{a:02x}"),
        }
    }
}

/// Attempt to parse a hexadecimal string into a `Color`
///
/// # Arguments
/// - `s`: A hexadecimal string representation of a color, i.e., "#DEADBEEF"
///
/// # Returns
/// - If successfully parsed, a `Color::Rgb` or `Color::Rgba`
///
/// # Errors
/// Can return a `ParseIntError` if it fails to convert any of the color channel strings into a
/// base 16 number
pub fn parse_rgb(s: &str) -> Result<Color, String> {
    if !s.starts_with('#') {
        return Err(format!("Invalid hex string: {s}"));
    }

    let hex = s.trim_start_matches('#');
    let len = hex.len();

    if len != 6 && len != 8 {
        return Err(format!(
            "Hex string must be in the format #RRGGBB or #RRGGBBAA instead of {s}"
        ));
    }

    let red = u8::from_str_radix(&hex[0..2], 16).unwrap_or_default();
    let green = u8::from_str_radix(&hex[2..4], 16).unwrap_or_default();
    let blue = u8::from_str_radix(&hex[4..6], 16).unwrap_or_default();

    if len == 8 {
        let alpha = u8::from_str_radix(&hex[6..8], 16).unwrap_or_default();
        Ok(Color::Rgba {
            r: red,
            g: green,
            b: blue,
            a: alpha,
        })
    } else {
        Ok(Color::Rgb {
            r: red,
            g: green,
            b: blue,
        })
    }
}

#[must_use]
/// Takes a foreground and background color and executes a blending algorithm for color smoothing
///
/// # Arguments
/// - `foreground`: The `Color` of the text
/// - `background`: The `Color` of the terminal background
///
/// # Returns
/// - The calculated blended color
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
            let alpha = f32::from(a) / 255.0;
            let inv_alpha = 1.0 - alpha;

            let r = (f32::from(r) * alpha + f32::from(background_r) * inv_alpha) as u8;
            let g = (f32::from(g) * alpha + f32::from(background_g) * inv_alpha) as u8;
            let b = (f32::from(b) * alpha + f32::from(background_b) * inv_alpha) as u8;

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
