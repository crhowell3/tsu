mod common;

use tsu::action::Action;
use tsu::editor::Mode;

use crate::common::EditorWrapper;

mod movement {
    use super::*;

    #[tokio::test]
    async fn test_cursor_movement() {
        let mut wrapper = EditorWrapper::with_content(
            "In the beginning was the Word\nand the Wod was with God\nand the Word was God",
        );

        wrapper.assert_cursor_position(0, 0);

        wrapper.execute_action(Action::MoveRight).await.unwrap();
        wrapper.assert_cursor_position(1, 0);

        wrapper.execute_action(Action::MoveLeft).await.unwrap();
        wrapper.assert_cursor_position(0, 0);

        wrapper.execute_action(Action::MoveDown).await.unwrap();
        wrapper.assert_cursor_position(0, 1);

        wrapper.execute_action(Action::MoveUp).await.unwrap();
        wrapper.assert_cursor_position(0, 0);
    }
}

mod modes {
    use super::*;

    #[tokio::test]
    async fn enter_insert_mode() {
        let mut wrapper = EditorWrapper::with_content(
            "In the beginning was the Word\nand the Wod was with God\nand the Word was God",
        );

        wrapper
            .execute_action(Action::EnterMode(Mode::Insert))
            .await
            .unwrap();
        wrapper.assert_mode(Mode::Insert);
    }

    #[tokio::test]
    async fn enter_command_mode() {
        let mut wrapper = EditorWrapper::with_content(
            "In the beginning was the Word\nand the Wod was with God\nand the Word was God",
        );

        wrapper
            .execute_action(Action::EnterMode(Mode::Command))
            .await
            .unwrap();
        wrapper.assert_mode(Mode::Command);
    }

    #[tokio::test]
    async fn enter_replace_mode() {
        let mut wrapper = EditorWrapper::with_content(
            "In the beginning was the Word\nand the Wod was with God\nand the Word was God",
        );

        wrapper
            .execute_action(Action::EnterMode(Mode::Replace))
            .await
            .unwrap();
        wrapper.assert_mode(Mode::Replace);
    }

    #[tokio::test]
    async fn enter_visual_mode() {
        let mut wrapper = EditorWrapper::with_content(
            "In the beginning was the Word\nand the Wod was with God\nand the Word was God",
        );

        wrapper
            .execute_action(Action::EnterMode(Mode::Visual))
            .await
            .unwrap();
        wrapper.assert_mode(Mode::Visual);
    }

    #[tokio::test]
    async fn is_normal_mode_on_startup() {
        let wrapper = EditorWrapper::with_content(
            "In the beginning was the Word\nand the Wod was with God\nand the Word was God",
        );

        wrapper.is_normal();
    }

    #[tokio::test]
    async fn change_back_to_normal_mode() {
        let mut wrapper = EditorWrapper::with_content(
            "In the beginning was the Word\nand the Wod was with God\nand the Word was God",
        );

        wrapper.is_normal();

        wrapper
            .execute_action(Action::EnterMode(Mode::Insert))
            .await
            .unwrap();

        wrapper.assert_mode(Mode::Insert);

        wrapper
            .execute_action(Action::EnterMode(Mode::Normal))
            .await
            .unwrap();
    }
}
