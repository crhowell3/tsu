use std::{collections::HashMap, path::PathBuf};

use serde::{Deserialize, Serialize};

use crate::action::Action;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum KeyAction {
    None,
    Single(Action),
    Multiple(Vec<Action>),
    Nested(HashMap<String, KeyAction>),
    Repeating(u16, Box<KeyAction>),
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Keys {
    #[serde(default)]
    pub normal: HashMap<String, KeyAction>,
    #[serde(default)]
    pub insert: HashMap<String, KeyAction>,
    #[serde(default)]
    pub command: HashMap<String, KeyAction>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Config {
    pub keys: Keys,
    pub theme: String,
    pub log_file: Option<String>,
    pub mouse_scroll_lines: Option<usize>,
}

impl Config {
    pub fn path(p: &str) -> PathBuf {
        std::env::home_dir()
            .unwrap()
            .join(".config")
            .join("tsu")
            .join(p)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::fs;

    #[test]
    fn test_parse_config() {
        let toml = fs::read_to_string("src/fixtures/config.toml").unwrap();
        let config: Config = toml::from_str(&toml).unwrap();
        println!("{config:#?}");
    }
}
