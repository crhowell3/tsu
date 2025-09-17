mod editor;

use crate::editor::Editor;

fn main() -> anyhow::Result<()> {
    let editor = Editor::new();

    editor.unwrap().run()?;

    Ok(())
}
