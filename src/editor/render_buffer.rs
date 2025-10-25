use crate::{
    color::{Color, blend_color},
    theme::{Style, Theme},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cell {
    pub c: char,
    pub style: Style,
}

#[derive(Debug)]
pub struct Change<'a> {
    pub x: usize,
    pub y: usize,
    pub cell: &'a Cell,
}

#[derive(Debug, Clone)]
pub struct RenderBuffer {
    pub cells: Vec<Cell>,
    pub width: usize,
    #[allow(unused)]
    pub height: usize,
}

impl RenderBuffer {
    #[must_use]
    /// Creates a new `RenderBuffer` with a default style
    ///
    /// # Arguments
    /// - `width`: The desired width of the buffer measured in "cells"
    /// - `height`: The desired height of the buffer measured in "cells"
    /// - `default_style`: Some default style for formatting the empty cells
    ///
    /// # Returns
    /// - A newly constructed `RenderBuffer` with a default style and no content
    pub fn new(width: usize, height: usize, default_style: &Style) -> Self {
        let cells = vec![
            Cell {
                c: ' ',
                style: default_style.clone(),
            };
            width * height
        ];

        Self {
            cells,
            width,
            height,
        }
    }

    #[must_use]
    /// Creates a new `RenderBuffer` with text
    ///
    /// # Arguments
    /// - `width`: The desired width of the buffer measured in "cells"
    /// - `height`: The desired height of the buffer measured in "cells"
    /// - `style`: Styling to apply to the cells based on the provided content
    /// - `contents`: A vector of strings representing the buffer's text
    ///
    /// # Returns
    /// - A constructed `RenderBuffer` with styling and textual content
    pub fn new_with_contents(
        width: usize,
        height: usize,
        style: &Style,
        contents: Vec<String>,
    ) -> Self {
        let mut cells = vec![];

        for line in contents {
            for c in line.chars() {
                cells.push(Cell {
                    c,
                    style: style.clone(),
                });
            }
            for _ in 0..width.saturating_sub(line.len()) {
                cells.push(Cell {
                    c: ' ',
                    style: style.clone(),
                });
            }
        }

        Self {
            cells,
            width,
            height,
        }
    }

    /// Clears the buffer by replacing all characters within the cells with whitespace
    pub fn clear(&mut self) {
        self.cells = vec![
            Cell {
                c: ' ',
                style: Style::default(),
            };
            self.width * self.height
        ];
    }

    /// Add a character to a buffer cell with appropriate styling and theme
    ///
    /// # Arguments
    /// - `x`: The x coordinate of the cell
    /// - `y`: The y coordinate of the cell
    /// - `c`: The character to emplace
    /// - `style`: The styling of the character and its encapsulating cell
    /// - `theme`: The theme to use for styling
    pub fn set_char(&mut self, x: usize, y: usize, c: char, style: &Style, theme: &Theme) {
        if x > self.width || y > self.height {
            return;
        }
        let position = (y * self.width) + x;
        if position >= self.cells.len() {
            return;
        }

        let background = style.background.map(|color| match color {
            Color::Rgba { r, g, b, a } => blend_color(
                Color::Rgba { r, g, b, a },
                theme
                    .style
                    .background
                    .unwrap_or(Color::Rgb { r: 0, g: 0, b: 0 }),
            ),
            _ => color,
        });

        self.cells[position] = Cell {
            c,
            style: Style {
                foreground: style.foreground,
                background,
                bold: style.bold,
                italic: style.italic,
            },
        };
    }

    /// Adds a string of text to the buffer with appropriate styling
    ///
    /// # Arguments
    /// - `x`: The x coordinate representing the start of the text's destination within the buffer
    /// - `y`: The y coordinate representing the start of the text's destination within the buffer
    /// - `text`: The text to emplace
    /// - `style`: The styling to apply to the text on a cell-wise basis
    pub fn set_text(&mut self, x: usize, y: usize, text: &str, style: &Style) {
        let pos = (y * self.width) + x;
        for (i, c) in text.chars().enumerate() {
            if x + i >= self.width {
                break;
            }
            if pos + i >= self.cells.len() {
                break;
            }
            self.cells[pos + i] = Cell {
                c,
                style: style.clone(),
            }
        }
    }

    #[must_use]
    /// Computes the differences between two `RenderBuffer`s
    ///
    /// # Arguments
    /// - `other`: The buffer being compared to the current buffer
    ///
    /// # Returns
    /// - A vector of `Change`s which contains information about which positions and cells differ
    pub fn diff(&self, other: &RenderBuffer) -> Vec<Change<'_>> {
        let mut changes = vec![];
        for (pos, cell) in self.cells.iter().enumerate() {
            if *cell != other.cells[pos] {
                let y = pos / self.width;
                let x = pos % self.width;

                changes.push(Change { x, y, cell });
            }
        }

        changes
    }
}
