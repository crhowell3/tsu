# Using tsu

## Modes

tsu is a modal text editor, meaning that there are different modes of operation in which certain
tasks or actions may be executed. These are the available modes:

- Normal: The default mode wherein the user can move around the buffer and execute edit commands.
- Command: Enables the user to execute built-in commands such as `quit` or `write`.
- Insert: Allows the user to input text into the current buffer.

## Buffers

Buffers are representations of the contents of a file which allow the user to input and manipulate
said contents. As of tsu v0.1.0, only one buffer per application session is supported. In the future,
tsu will have support for multiple buffers per session.

## Motions

A "motion" is a command that is usually comprised of some combination of keypresses that executes a
corresponding action, such as moving the cursor or modifying text in the current buffer. The default
keybinds for motions are configurable. See [Keymap](usage/keymap.md) for the default key mapping as
well as the list of configurable actions.
