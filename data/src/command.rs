#[derive(Debug, Clone)]
pub enum Command {
    Internal(Internal),
}

#[derive(Debug, Clone)]
pub enum Internal {}
