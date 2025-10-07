use crate::{editor::Point, window::Window};

#[derive(Debug, Clone)]
pub enum Split {
    Window(Window),
    Horizontal {
        top: Box<Split>,
        bottom: Box<Split>,
        ratio: f32,
    },
    Vertical {
        left: Box<Split>,
        right: Box<Split>,
        ratio: f32,
    },
}

impl Split {
    pub fn new_window(buffer_index: usize, position: Point, size: (usize, usize)) -> Self {
        Split::Window(Window::new(buffer_index, position, size))
    }

    pub fn windows(&self) -> Vec<&Window> {
        match self {
            Split::Window(w) => vec![w],
            Split::Horizontal { top, bottom, .. } => {
                let mut windows = top.windows();
                windows.extend(bottom.windows());
                windows
            }
            Split::Vertical { left, right, .. } => {
                let mut windows = left.windows();
                windows.extend(right.windows());
                windows
            }
        }
    }

    pub fn windows_mut(&mut self) -> Vec<&mut Window> {
        match self {
            Split::Window(w) => vec![w],
            Split::Horizontal { top, bottom, .. } => {
                let mut windows = top.windows_mut();
                windows.extend(bottom.windows_mut());
                windows
            }
            Split::Vertical { left, right, .. } => {
                let mut windows = left.windows_mut();
                windows.extend(right.windows_mut());
                windows
            }
        }
    }

    pub fn layout(&mut self, position: Point, size: (usize, usize)) {
        match self {
            Split::Window(w) => {
                w.position = position;
                w.size = size;
            }
            Split::Horizontal { top, bottom, ratio } => {
                let available_height = size.1.saturating_sub(1);
                let split_y = (available_height as f32 * *ratio) as usize;

                top.layout(position, (size.0, split_y));
                bottom.layout(
                    Point::new(position.x, position.y + split_y + 1),
                    (size.0, available_height - split_y),
                );
            }
            Split::Vertical { left, right, ratio } => {
                let available_width = size.0.saturating_sub(1);
                let split_x = (available_width as f32 * *ratio) as usize;

                left.layout(position, (split_x, size.1));
                right.layout(
                    Point::new(position.x + split_x + 1, position.y),
                    (available_width - split_x, size.1),
                );
            }
        }
    }
}

pub struct WindowManager {
    root: Split,
    active_window_id: usize,
}

impl WindowManager {
    pub fn new(buffer_index: usize, terminal_size: (usize, usize)) -> Self {
        let mut root = Split::new_window(
            buffer_index,
            Point::new(0, 0),
            (terminal_size.0, terminal_size.1.saturating_sub(2)),
        );

        if let Split::Window(w) = &mut root {
            w.active = true;
        }

        Self {
            root,
            active_window_id: 0,
        }
    }

    pub fn active_window(&self) -> Option<&Window> {
        self.root.windows().get(self.active_window_id).copied()
    }

    pub fn active_window_mut(&mut self) -> Option<&mut Window> {
        let mut current_id = 0;
        Self::get_window_mut_recursive(&mut self.root, &mut current_id, self.active_window_id)
    }

    fn get_window_mut_recursive<'a>(
        node: &'a mut Split,
        current_id: &mut usize,
        target_id: usize,
    ) -> Option<&'a mut Window> {
        match node {
            Split::Window(window) => {
                if *current_id == target_id {
                    Some(window)
                } else {
                    *current_id += 1;
                    None
                }
            }
            Split::Horizontal { top, bottom, .. } => {
                if let Some(window) = Self::get_window_mut_recursive(top, current_id, target_id) {
                    return Some(window);
                }
                Self::get_window_mut_recursive(bottom, current_id, target_id)
            }
            Split::Vertical { left, right, .. } => {
                if let Some(window) = Self::get_window_mut_recursive(left, current_id, target_id) {
                    return Some(window);
                }
                Self::get_window_mut_recursive(right, current_id, target_id)
            }
        }
    }

    pub fn windows(&self) -> Vec<&Window> {
        self.root.windows()
    }

    pub fn windows_mut(&mut self) -> Vec<&mut Window> {
        self.root.windows_mut()
    }

    pub fn resize(&mut self, terminal_size: (usize, usize)) {
        self.root.layout(
            Point::new(0, 0),
            (terminal_size.0, terminal_size.1.saturating_sub(2)),
        );
    }

    pub fn active_window_id(&self) -> usize {
        self.active_window_id
    }

    pub fn set_active(&mut self, window_id: usize) {
        for window in self.root.windows_mut() {
            window.active = false;
        }

        if let Some(window) = self.root.windows_mut().get_mut(window_id) {
            window.active = true;
            self.active_window_id = window_id;
        }
    }
}
