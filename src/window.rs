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
    #[must_use]
    /// Constructs a new window with a buffer index, a position, and a size
    ///
    /// # Arguments
    /// - `buffer_index`: The index of the buffer that the window will encapsulate
    /// - `position`: The position of the window
    /// - `size`: The size of the window
    ///
    /// # Returns
    /// - A newly constructed window
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

    #[must_use]
    /// Retrieves the interior width of the window
    ///
    /// # Returns
    /// - The interior width of the window
    pub fn inner_width(&self) -> usize {
        self.size.0
    }

    #[must_use]
    /// Retrieves the interior height of the window
    ///
    /// # Returns
    /// - The interior height of the window
    pub fn inner_height(&self) -> usize {
        self.size.1
    }

    #[must_use]
    /// Checks if the provided position is within the current window
    ///
    /// # Arguments
    /// - `x`: The x coordinate of the position
    /// - `y`: The y coordinate of the position
    ///
    /// # Returns
    /// - `true` if the position is within the window, `false` otherwise
    pub fn contains_position(&self, x: usize, y: usize) -> bool {
        x >= self.position.x
            && x < self.position.x + self.size.0
            && y >= self.position.y
            && y < self.position.y + self.size.1
    }

    #[must_use]
    /// Transforms terminal coordinates to local coordinates, i.e., position relative to the window
    ///
    /// # Arguments
    /// - `terminal_x`: The x coordinate relative to the terminal
    /// - `terminal_y`: The y coordinate relative to the terminal
    ///
    /// # Returns
    /// - Local coordinates if the terminal coordinates provided are contained within the window
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

    #[must_use]
    /// Transforms local coordinates to terminal coordinates, i.e., position relative to the terminal
    ///
    /// # Arguments
    /// - `local_x`: The x coordinate relative to the window
    /// - `local_y`: The y coordinate relative to the window
    ///
    /// # Returns
    /// - A tuple of terminal coordinates in the format (x, y)
    pub fn local_to_terminal_transform(&self, local_x: usize, local_y: usize) -> (usize, usize) {
        (self.position.x + local_x, self.position.y + local_y)
    }
}
