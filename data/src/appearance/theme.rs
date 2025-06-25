use std::path::PathBuf;

use iced_core::Color;
use palette::rgb::{Rgb, Rgba};
use palette::{FromColor, Hsva, Okhsl, Srgba};
use rand::prelude::*;
use rand_chacha::ChaChaRng;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::fs;

const DEFAULT_THEME_NAME: &str = "gruvbox";
const DEFAULT_THEME_CONTENT: &str = include_str!("../../../assets/themes/gruvbox.toml");

#[derive(Debug, Clone)]
pub struct Theme {
    pub name: String,
    pub colors: Colors,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            name: DEFAULT_THEME_NAME.to_string(),
            colors: Colors::default(),
        }
    }
}

impl Theme {
    #[must_use]
    pub fn new(name: String, colors: Colors) -> Self {
        Theme { name, colors }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Colors {
    #[serde(default)]
    pub general: General,
    #[serde(default)]
    pub text: Text,
    #[serde(default)]
    pub buttons: Buttons,
}

impl Colors {
    /// # Errors
    ///
    /// Will return `Error` if unable to write data to the provided file path.
    pub async fn save(self, path: PathBuf) -> Result<(), Error> {
        let content = toml::to_string(&self)?;

        fs::write(path, &content).await?;

        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to serialize theme to toml: {0}")]
    Encode(#[from] toml::ser::Error),
    #[error("Failed to write theme file: {0}")]
    Write(#[from] std::io::Error),
}

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize)]
pub struct Buttons {
    #[serde(default)]
    pub primary: Button,
    #[serde(default)]
    pub secondary: Button,
}

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize)]
pub struct Button {
    #[serde(default = "default_transparent", with = "color_serde")]
    pub background: Color,
    #[serde(default = "default_transparent", with = "color_serde")]
    pub background_hover: Color,
    #[serde(default = "default_transparent", with = "color_serde")]
    pub background_selected: Color,
    #[serde(default = "default_transparent", with = "color_serde")]
    pub background_selected_hover: Color,
}

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize)]
pub struct General {
    #[serde(default = "default_transparent", with = "color_serde")]
    pub background: Color,
    #[serde(default = "default_transparent", with = "color_serde")]
    pub border: Color,
    #[serde(default = "default_transparent", with = "color_serde")]
    pub horizontal_rule: Color,
    #[serde(default = "default_transparent", with = "color_serde")]
    pub unread_indicator: Color,
}

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize)]
pub struct Text {
    #[serde(default = "default_transparent", with = "color_serde")]
    pub primary: Color,
    #[serde(default = "default_transparent", with = "color_serde")]
    pub secondary: Color,
    #[serde(default = "default_transparent", with = "color_serde")]
    pub tertiary: Color,
    #[serde(default = "default_transparent", with = "color_serde")]
    pub success: Color,
    #[serde(default = "default_transparent", with = "color_serde")]
    pub error: Color,
}

impl Default for Colors {
    fn default() -> Self {
        toml::from_str(DEFAULT_THEME_CONTENT).expect("parse default theme")
    }
}

#[must_use]
pub fn hex_to_color(hex: &str) -> Option<Color> {
    if hex.len() == 7 || hex.len() == 9 {
        let hash = &hex[0..1];
        let r = u8::from_str_radix(&hex[1..3], 16);
        let g = u8::from_str_radix(&hex[3..5], 16);
        let b = u8::from_str_radix(&hex[5..7], 16);
        let a = (hex.len() == 9)
            .then(|| u8::from_str_radix(&hex[7..9], 16).ok())
            .flatten();

        return match (hash, r, g, b, a) {
            ("#", Ok(r), Ok(g), Ok(b), None) => Some(Color {
                r: f32::from(r) / 255.0,
                g: f32::from(g) / 255.0,
                b: f32::from(b) / 255.0,
                a: 1.0,
            }),
            ("#", Ok(r), Ok(g), Ok(b), Some(a)) => Some(Color {
                r: f32::from(r) / 255.0,
                g: f32::from(g) / 255.0,
                b: f32::from(b) / 255.0,
                a: f32::from(a) / 255.0,
            }),
            _ => None,
        };
    }

    None
}

#[must_use]
pub fn color_to_hex(color: Color) -> String {
    use std::fmt::Write;

    let mut hex = String::with_capacity(9);

    let [r, g, b, a] = color.into_rgba8();

    let _ = write!(&mut hex, "#");
    let _ = write!(&mut hex, "{r:02X}");
    let _ = write!(&mut hex, "{g:02X}");
    let _ = write!(&mut hex, "{b:02X}");

    if a < u8::MAX {
        let _ = write!(&mut hex, "{a:02X}");
    }

    hex
}

pub fn alpha_color_calculate(
    min_alpha: f32,
    max_alpha: f32,
    background: Color,
    foreground: Color,
) -> Color {
    alpha_color(
        foreground,
        min_alpha + to_hsl(background).lightness * (max_alpha - min_alpha),
    )
}

pub fn randomize_color(original_color: Color, seed: &str) -> Color {
    let seed_hash = seahash::hash(seed.as_bytes());

    let mut rng = ChaChaRng::seed_from_u64(seed_hash);

    // Convert the original color to HSL
    let original_hsl = to_hsl(original_color);

    let randomized_hue: f32 = rng.random_range(0.0..=360.0);
    let randomized_hsl = Okhsl::new(
        randomized_hue,
        original_hsl.saturation,
        original_hsl.lightness,
    );

    from_hsl(randomized_hsl)
}

pub fn to_hsl(color: Color) -> Okhsl {
    let mut hsl = Okhsl::from_color(to_rgb(color));
    if hsl.saturation.is_nan() {
        hsl.saturation = Okhsl::max_saturation();
    }

    hsl
}

pub fn to_hsva(color: Color) -> Hsva {
    Hsva::from_color(to_rgba(color))
}

pub fn from_hsva(color: Hsva) -> Color {
    to_color(Srgba::from_color(color))
}

pub fn from_hsl(hsl: Okhsl) -> Color {
    to_color(Srgba::from_color(hsl))
}

pub fn alpha_color(color: Color, alpha: f32) -> Color {
    Color { a: alpha, ..color }
}

fn default_transparent() -> Color {
    Color::TRANSPARENT
}

fn to_rgb(color: Color) -> Rgb {
    Rgb {
        red: color.r,
        green: color.g,
        blue: color.b,
        ..Rgb::default()
    }
}

fn to_rgba(color: Color) -> Rgba {
    Rgba {
        alpha: color.a,
        color: to_rgb(color),
    }
}

fn to_color(rgba: Rgba) -> Color {
    Color {
        r: rgba.color.red,
        g: rgba.color.green,
        b: rgba.color.blue,
        a: rgba.alpha,
    }
}

mod color_serde {
    use iced_core::Color;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Color, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(String::deserialize(deserializer)
            .map(|hex| super::hex_to_color(&hex))?
            .unwrap_or(Color::TRANSPARENT))
    }

    pub fn serialize<S>(color: &Color, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        super::color_to_hex(*color).serialize(serializer)
    }
}
