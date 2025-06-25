use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum Pane {
    Split {
        axis: Axis,
        ratio: f32,
        a: Box<Pane>,
        b: Box<Pane>,
    },
    Empty,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub enum Axis {
    Horizontal,
    Vertical,
}
