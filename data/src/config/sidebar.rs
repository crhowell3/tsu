use serde::Deserialize;

#[derive(Debug, Copy, Clone, Deserialize, Default)]
pub struct Sidebar {
    #[serde(default)]
    pub max_width: Option<u16>,
    #[serde(default)]
    pub position: Position,
}

#[derive(Debug, Copy, Clone, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum Position {
    #[default]
    Left,
    Right,
}
