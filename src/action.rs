use serde::{Deserialize, Serialize};

use crate::config::KeyAction;
use crate::editor::Mode;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Action {
    // Buffer actions
    Quit(bool),
    Save,
    SaveAs(String),
    Undo,
    UndoMultiple(Vec<Action>),
    CenterView,

    // Cursor movement
    MoveUp,
    MoveDown,
    MoveLeft,
    MoveRight,
    MoveTo(usize, usize),
    MoveToTop,
    MoveToBottom,
    MoveToLineEnd,
    MoveToLineStart,
    MoveLineToViewCenter,
    MoveLineToViewBottom,
    MoveViewDownOneLine,
    MoveViewUpOneLine,
    MoveToBottomOfBuffer,
    MoveToTopOfBuffer,

    // Page movement
    PageUp,
    PageDown,

    // Commands
    Command(String),

    // Text editing
    InsertCharAtCursor(char),
    RemoveCharAt(usize, usize),
    InsertLineAbove,
    InsertLineBelow,
    InsertLineAt(usize, Option<String>),
    InsertNewLine,
    DeletePreviousChar,
    DeleteCharAtCursor,
    DeleteCurrentLine,
    DeleteLineAt(usize),

    GoToLine(usize),

    // Misc
    EnterMode(Mode),
    SetWaitingKey(Box<KeyAction>),
}
