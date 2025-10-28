use serde::{Deserialize, Serialize};

use crate::config::KeyAction;
use crate::editor::Mode;

#[derive(Debug, Serialize, Deserialize, Clone)]
/// The collection of handled editor actions
///
/// This is primarily leveraged by the main editor processing loop for mapping user input sequences to handleable actions
pub enum Action {
    /// Closes the application
    Quit(bool),
    /// Writes the current buffer to a file
    Save,
    /// Writes the current buffer to the file provided as an argument to the ':w' command
    SaveAs(String),
    /// Reverses the previous undoable action
    Undo,
    /// Reverses multiple previous undoable actions simultaneously
    UndoMultiple(Vec<Action>),
    /// Moves the current line to the center of the view
    CenterView,

    /// Move the cursor up
    MoveUp,
    /// Move the cursor down
    MoveDown,
    /// Move the cursor left
    MoveLeft,
    /// Move the cursor right
    MoveRight,
    /// Move to a specific (x, y) position in the buffer
    MoveTo(usize, usize),
    /// Move the cursor to the top of the buffer
    MoveToTop,
    /// Move the cursor to the bottom of the buffer
    MoveToBottom,
    /// Move the cursor to the end of the current line
    MoveToLineEnd,
    /// Move the cursor to the beginning of the current line
    MoveToLineStart,
    /// Move the current line to the center of the view
    MoveLineToViewCenter,
    /// Move the current line to the bottom of the view
    MoveLineToViewBottom,
    /// Move the viewport down one line
    MoveViewDownOneLine,
    /// Move the viewport up one line
    MoveViewUpOneLine,
    /// Move the cursor to the bottom of the buffer
    MoveToBottomOfBuffer,
    /// Move the cursor to the top of the buffer
    MoveToTopOfBuffer,

    /// Move the view up one "page"
    PageUp,
    /// Move the view down one "page"
    PageDown,

    /// Run a command given a string input in command mode
    Command(String),

    /// Insert a character at the current location of the cursor
    InsertCharAtCursor(char),
    /// Remove a character at the specified position
    RemoveCharAt(usize, usize),
    /// Insert a line above the cursor
    InsertLineAbove,
    /// Insert a line below the cursor
    InsertLineBelow,
    /// Insert a line at the specified y coordinate
    InsertLineAt(usize, Option<String>),
    /// Insert a new line character ('\n')
    InsertNewLine,

    /// Replace the contents of the line at the specified y coordinate with another string
    ReplaceLineAt(usize, String),

    /// Delete the character behind the cursor
    DeletePreviousChar,
    /// Delete the character at the cursor
    DeleteCharAtCursor,
    /// Delete the line where the cursor is currently located
    DeleteCurrentLine,
    /// Delete the line at the specified y coordinate
    DeleteLineAt(usize),

    /// Go to the specified line provided in command mode
    GoToLine(usize),

    /// Change to mode
    EnterMode(Mode),
    /// Store the first key in a multi-character command sequence
    SetWaitingKey(Box<KeyAction>),
    /// Force a complete redraw of the editor
    Redraw,
}
