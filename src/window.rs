use crate::editor::Point;

#[derive(Debug, Clone, Copy)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Debug, Clone)]
pub struct Window {
    pub buffer_index: usize,
    pub position: Point,
    pub size: (usize, usize),
    pub vtop: usize,
    pub vleft: usize,
    pub cursor_x: usize,
    pub cursor_y: usize,
    pub active: bool,
    pub vx: usize,
}

impl Window {
    pub fn new(buffer_index: usize, position: Point, size: (usize, usize)) -> Self {
        Self {
            buffer_index,
            position,
            size,
            vtop: 0,
            vleft: 0,
            cursor_x: 0,
            cursor_y: 0,
            active: false,
            vx: 0,
        }
    }

    pub fn inner_width(&self) -> usize {
        self.size.0
    }

    pub fn inner_height(&self) -> usize {
        self.size.1
    }

    pub fn contains_position(&self, x: usize, y: usize) -> bool {
        x >= self.position.x
            && x < self.position.x + self.size.0
            && y >= self.position.y
            && y <= self.position.y + self.size.1
    }

    pub fn terminal_to_local_transform(
        &self,
        terminal_x: usize,
        terminal_y: usize,
    ) -> Option<(usize, usize)> {
        if self.contains_position(terminal_x, terminal_y) {
            Some((terminal_x - self.position.x, terminal_y - self.position.y))
        } else {
            None
        }
    }

    pub fn local_to_terminal_transform(&self, local_x: usize, local_y: usize) -> (usize, usize) {
        (self.position.x + local_x, self.position.y + local_y)
    }
}
