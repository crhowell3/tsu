use tsu::{
    buffer::Buffer,
    config::Config,
    editor::{Action, Editor, Mode},
    theme::Theme,
};

pub struct EditorWrapper {
    pub editor: Editor,
}

impl EditorWrapper {
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
        let editor = Editor::with_size(80, 24, config, theme, buffer).unwrap();

        Self { editor }
    }

    pub async fn execute_action(&mut self, action: Action) -> anyhow::Result<()> {
        self.editor.test_execute_action(action).await
    }

    pub fn cursor_position(&self) -> (usize, usize) {
        self.editor.test_cursor_position()
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
}
