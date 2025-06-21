use std::io;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Editor {}

impl Editor {
    pub fn load() -> Result<Self, Error> {
        Ok(Self {})
    }

    pub async fn save(self) -> Result<(), Error> {
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] io::Error),
}
