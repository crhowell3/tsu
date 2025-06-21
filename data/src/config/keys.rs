use serde::Deserialize;

use crate::shortcut::{KeyBind, Shortcut, shortcut};

#[derive(Debug, Clone, Deserialize)]
pub struct Keyboard {
    #[serde(default = "KeyBind::move_up")]
    pub move_up: KeyBind,
    #[serde(default = "KeyBind::move_down")]
    pub move_down: KeyBind,
    #[serde(default = "KeyBind::move_left")]
    pub move_left: KeyBind,
    #[serde(default = "KeyBind::move_right")]
    pub move_right: KeyBind,
    #[serde(default = "KeyBind::toggle_sidebar")]
    pub toggle_sidebar: KeyBind,
    #[serde(default = "KeyBind::toggle_fullscreen")]
    pub toggle_fullscreen: KeyBind,
    #[serde(default = "KeyBind::theme_editor")]
    pub theme_editor: KeyBind,
    #[serde(default)]
    pub quit_application: Option<KeyBind>,
}

impl Default for Keyboard {
    fn default() -> Self {
        Self {
            move_up: KeyBind::move_up(),
            move_down: KeyBind::move_down(),
            move_left: KeyBind::move_left(),
            move_right: KeyBind::move_right(),
            toggle_sidebar: KeyBind::toggle_sidebar(),
            toggle_fullscreen: KeyBind::toggle_fullscreen(),
            theme_editor: KeyBind::theme_editor(),
            quit_application: None,
        }
    }
}

impl Keyboard {
    pub fn shortcuts(&self) -> Vec<Shortcut> {
        use crate::shortcut::Command::*;

        let mut shortcuts = vec![
            shortcut(self.move_up.clone(), MoveUp),
            shortcut(self.move_down.clone(), MoveDown),
            shortcut(self.move_left.clone(), MoveLeft),
            shortcut(self.move_right.clone(), MoveRight),
            shortcut(self.toggle_sidebar.clone(), ToggleSidebar),
            shortcut(self.toggle_fullscreen.clone(), ToggleFullscreen),
            shortcut(self.theme_editor.clone(), ThemeEditor),
        ];

        if let Some(quit_application) = self.quit_application.clone() {
            shortcuts.push(shortcut(quit_application, QuitApplication));
        }

        shortcuts
    }
}
