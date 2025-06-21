use std::path::PathBuf;
use std::{str, string};

use iced_core::font;
use serde::{Deserialize, Deserializer};
use thiserror::Error;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::ReadDirStream;

pub use self::keys::Keyboard;
use crate::appearance::theme::Colors;
use crate::appearance::{self, Appearance};
use crate::environment::config_dir;
use crate::{Theme, environment};

pub mod keys;

const CONFIG_TEMPLATE: &str = include_str!("../../config.toml");
const DEFAULT_THEME_NAME: &str = "gruvbox";

#[derive(Debug, Clone, Default)]
pub struct Config {
    pub appearance: Appearance,
    pub font: Font,
    pub keyboard: Keyboard,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Font {
    pub family: Option<String>,
    pub size: Option<u8>,
    #[serde(
        default = "default_font_weight",
        deserialize_with = "deserialize_font_weight_from_string"
    )]
    pub weight: font::Weight,
    #[serde(
        default,
        deserialize_with = "deserialize_optional_font_weight_from_string"
    )]
    pub bold_weight: Option<font::Weight>,
}

fn deserialize_font_weight_from_string<'de, D>(deserializer: D) -> Result<font::Weight, D::Error>
where
    D: Deserializer<'de>,
{
    let string = String::deserialize(deserializer)?;

    match string.as_ref() {
        "thin" => Ok(font::Weight::Thin),
        "extra-light" => Ok(font::Weight::ExtraLight),
        "light" => Ok(font::Weight::Light),
        "normal" => Ok(font::Weight::Normal),
        "medium" => Ok(font::Weight::Medium),
        "semibold" => Ok(font::Weight::Semibold),
        "bold" => Ok(font::Weight::Bold),
        "extra-bold" => Ok(font::Weight::ExtraBold),
        "black" => Ok(font::Weight::Black),
        _ => Err(serde::de::Error::invalid_value(
            serde::de::Unexpected::Str(&string),
            &"expected one of font weight names: \
              \"thin\", \
              \"extra-light\", \
              \"light\", \
              \"normal\", \
              \"medium\", \
              \"semibold\", \
              \"bold\", \
              \"extra-bold\", and \
              \"black\"",
        )),
    }
}

fn deserialize_optional_font_weight_from_string<'de, D>(
    deserializer: D,
) -> Result<Option<font::Weight>, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Some(deserialize_font_weight_from_string(deserializer)?))
}

fn default_font_weight() -> font::Weight {
    font::Weight::Normal
}

impl Config {
    pub fn config_dir() -> PathBuf {
        let dir = environment::config_dir();

        if !dir.exists() {
            std::fs::create_dir_all(dir.as_path())
                .expect("expected permissions to create config directory");
        }

        dir
    }

    pub fn themes_dir() -> PathBuf {
        let dir = Self::config_dir().join("themes");

        if !dir.exists() {
            std::fs::create_dir_all(dir.as_path())
                .expect("expected permissions to create themes directory");
        }

        dir
    }

    pub fn path() -> PathBuf {
        Self::config_dir().join(environment::CONFIG_FILE_NAME)
    }

    pub async fn load() -> Result<Self, Error> {
        use tokio::fs;

        #[derive(Deserialize, Debug)]
        #[serde(untagged)]
        pub enum ThemeKeys {
            Static(String),
            Dynamic { light: String, dark: String },
        }

        impl Default for ThemeKeys {
            fn default() -> Self {
                Self::Static(String::default())
            }
        }

        impl ThemeKeys {
            pub fn keys(&self) -> (&str, Option<&str>) {
                match self {
                    ThemeKeys::Static(manual) => (manual, None),
                    ThemeKeys::Dynamic { light, dark } => (light, Some(dark)),
                }
            }
        }

        #[derive(Deserialize)]
        pub struct Configuration {
            #[serde(default)]
            pub theme: ThemeKeys,
            #[serde(default)]
            pub font: Font,
            #[serde(default)]
            pub keyboard: Keyboard,
        }

        let path = Self::path();
        if !path.try_exists()? {
            return Err(Error::ConfigMissing {
                has_yaml_config: has_yaml_config()?,
            });
        }
        let content = fs::read_to_string(path)
            .await
            .map_err(|e| Error::LoadConfigFile(e.to_string()))?;

        let Configuration {
            theme,
            font,
            keyboard,
        } = toml::from_str(content.as_ref()).map_err(|e| Error::Parse(e.to_string()))?;

        let appearance = Self::load_appearance(theme.keys())
            .await
            .unwrap_or_default();

        Ok(Config {
            appearance,
            font,
            keyboard,
        })
    }

    async fn load_appearance(theme_keys: (&str, Option<&str>)) -> Result<Appearance, Error> {
        use tokio::fs;

        #[derive(Deserialize)]
        #[serde(untagged)]
        pub enum Data {
            V1 {
                #[serde(rename = "name")]
                _name: String,
            },
            V2(Colors),
        }

        let read_entry = |entry: fs::DirEntry| async move {
            let content = fs::read_to_string(entry.path()).await.ok()?;

            let data: Data = toml::from_str(content.as_ref()).ok()?;
            let name = entry.path().file_stem()?.to_string_lossy().to_string();

            match data {
                Data::V1 { .. } => None,
                Data::V2(colors) => Some(Theme::new(name, colors)),
            }
        };

        let mut all = vec![];
        let mut first_theme = Theme::default();
        let mut second_theme = theme_keys.1.map(|_| Theme::default());
        let mut has_tsu_theme = false;

        let mut stream = ReadDirStream::new(fs::read_dir(Self::themes_dir()).await?);
        while let Some(entry) = stream.next().await {
            let Ok(entry) = entry else {
                continue;
            };

            let Some(file_name) = entry.file_name().to_str().map(String::from) else {
                continue;
            };

            if let Some(file_name) = file_name.strip_suffix(".toml") {
                if let Some(theme) = read_entry(entry).await {
                    if file_name == theme_keys.0 {
                        first_theme = theme.clone();
                    }

                    if Some(file_name) == theme_keys.1 {
                        second_theme = Some(theme.clone());
                    }

                    if file_name.to_lowercase() == DEFAULT_THEME_NAME {
                        has_tsu_theme = true;
                    }

                    all.push(theme);
                }
            }
        }

        if !has_tsu_theme {
            all.push(Theme::default());
        }

        let selected = if let Some(second_theme) = second_theme {
            appearance::Selected::dynamic(first_theme, second_theme)
        } else {
            appearance::Selected::specific(first_theme)
        };

        Ok(Appearance { selected, all })
    }

    pub fn create_initial_config() {
        let config_file = Self::path();
        if config_file.exists() {
            return;
        }

        let config_string = CONFIG_TEMPLATE;
        let config_bytes = config_string.as_bytes();

        let config_path = Self::config_dir().join("config.toml");

        let _ = std::fs::write(config_path, config_bytes);
    }
}

fn has_yaml_config() -> Result<bool, Error> {
    Ok(config_dir().join("config.yaml").try_exists()?)
}

fn default_tooltip() -> bool {
    true
}

#[derive(Debug, Error, Clone)]
pub enum Error {
    #[error("config could not be read: {0}")]
    LoadConfigFile(String),

    #[error("{0}")]
    Io(String),
    #[error("{0}")]
    Parse(String),
    #[error("UTF8 parsing error: {0}")]
    StrUtf8Error(#[from] str::Utf8Error),
    #[error("UTF8 parsing error: {0}")]
    StringUtf8Error(#[from] string::FromUtf8Error),

    #[error("Config does not exist")]
    ConfigMissing { has_yaml_config: bool },
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error.to_string())
    }
}
