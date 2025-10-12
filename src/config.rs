use std::{collections::HashMap, path::PathBuf};

use serde::{Deserialize, Serialize};

use crate::action::Action;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
/// Representation of the types of key actions
pub enum KeyAction {
    /// No key action
    None,
    /// A single key action corresponding to an `Action`
    Single(Action),
    /// Multiple key actions corresponding to multiple `Action`s
    Multiple(Vec<Action>),
    /// Key actions wrapped inside other key actions
    Nested(HashMap<String, KeyAction>),
    /// A single key action repeated n times
    Repeating(u16, Box<KeyAction>),
}

#[derive(Debug, Serialize, Deserialize, Default)]
/// Maps keys to `KeyAction`s
pub struct Keys {
    #[serde(default)]
    pub normal: HashMap<String, KeyAction>,
    #[serde(default)]
    pub insert: HashMap<String, KeyAction>,
    #[serde(default)]
    pub command: HashMap<String, KeyAction>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
/// Pseudo-schema for the configuration file format
pub struct Config {
    /// Mapping of keys to their corresponding key actions
    pub keys: Keys,
    /// Path to theme file to use
    pub theme: String,
    /// Optional path to log file
    pub log_file: Option<String>,
    /// Optionally increase the amount of lines scrolled per mouse scroll
    pub mouse_scroll_lines: Option<usize>,
    #[serde(default = "default_false")]
    pub window_borders_ascii: bool,
}

impl Config {
    #[must_use]
    /// Construct the path to the configuration file
    ///
    /// # Arguments
    /// - `p`: Name of the configuration file
    ///
    /// # Panics
    /// This function might panic if querying the home directory returns an `Err`
    pub fn path(p: &str) -> PathBuf {
        std::env::home_dir()
            .unwrap()
            .join(".config")
            .join("tsu")
            .join(p)
    }
}

pub fn default_false() -> bool {
    false
}

#[cfg(test)]
mod test {
    use super::*;
    use std::fs;

    #[test]
    fn test_parse_config() {
        let toml = fs::read_to_string("default_config.toml").unwrap();
        let config: Config = toml::from_str(&toml).unwrap();
        println!("{config:#?}");
    }
}
