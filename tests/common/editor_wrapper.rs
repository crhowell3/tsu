use tsu::{
    action::Action,
    buffer::Buffer,
    config::Config,
    editor::{Editor, Mode},
    ext::EditorTestExt,
    theme::Theme,
};

pub struct EditorWrapper {
    pub editor: Editor,
}

impl EditorWrapper {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self::with_content("")
    }

    pub fn with_content(content: &str) -> Self {
        let buffer = Buffer::new(None, content.to_string());
        Self::with_buffer(buffer)
    }

    pub fn with_buffer(buffer: Buffer) -> Self {
        let config = Config::default();
        let theme = Theme::default();
        let editor = Editor::with_size(80, 24, config, theme, vec![buffer]).unwrap();

        Self { editor }
    }

    pub async fn execute_action(&mut self, action: Action) -> anyhow::Result<()> {
        self.editor.test_execute_action(action).await
    }

    pub fn cursor_position(&self) -> (usize, usize) {
        self.editor.test_cursor_position()
    }

    pub fn mode(&self) -> Mode {
        self.editor.test_mode()
    }

    #[allow(dead_code)]
    pub fn buffer_contents(&self) -> String {
        self.editor.test_buffer_contents()
    }

    pub fn is_normal(&self) -> bool {
        self.editor.test_is_normal()
    }

    pub fn assert_cursor_position(&self, x: usize, y: usize) {
        let (pos_x, pos_y) = self.cursor_position();
        assert_eq!(
            (pos_x, pos_y),
            (x, y),
            "Expected cursor at ({}, {}) but was found at ({}, {})",
            x,
            y,
            pos_x,
            pos_y
        );
    }

    pub fn assert_mode(&self, mode: Mode) {
        assert_eq!(
            self.mode(),
            mode,
            "Expected mode {:?}, but was {:?}",
            mode,
            self.mode()
        );
    }
}
