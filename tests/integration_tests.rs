mod editor_wrapper;

use tsu::editor::Action;

use crate::editor_wrapper::EditorWrapper;

#[tokio::test]
async fn test_cursor_movement() {
    let mut wrapper = EditorWrapper::with_content(
        "In the beginning was the Word\nand the Wod was with God\nand the Word was God",
    );

    wrapper.assert_cursor_position(0, 0);

    wrapper.execute_action(Action::MoveRight).await.unwrap();
    wrapper.assert_cursor_position(1, 0);
}
