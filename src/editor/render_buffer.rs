use crate::{
    color::{Color, blend_color},
    log,
    theme::{Style, Theme},
};

use super::Point;

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
    pub fn new(width: usize, height: usize, default_style: Style) -> Self {
        let cells = vec![
            Cell {
                c: ' ',
                style: default_style,
            };
            width * height
        ];

        Self {
            cells,
            width,
            height,
        }
    }

    #[allow(dead_code)]
    pub fn new_with_contents(
        width: usize,
        height: usize,
        style: Style,
        contents: Vec<String>,
    ) -> Self {
        let mut cells = vec![];

        for line in contents {
            for c in line.chars() {
                cells.push(Cell { c, style });
            }
            for _ in 0..width.saturating_sub(line.len()) {
                cells.push(Cell { c: ' ', style });
            }
        }

        Self {
            cells,
            width,
            height,
        }
    }

    pub fn clear(&mut self) {
        self.cells = vec![
            Cell {
                c: ' ',
                style: Style::default(),
            };
            self.width * self.height
        ];
    }

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

    pub fn set_text(&mut self, x: usize, y: usize, text: &str, style: &Style) {
        let pos = (y * self.width) + x;
        for (i, c) in text.chars().enumerate() {
            if x + i >= self.width {
                break;
            }
            if pos + i >= self.cells.len() {
                break;
            }
            self.cells[pos + i] = Cell { c, style: *style }
        }
    }

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
