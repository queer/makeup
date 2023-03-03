use async_trait::async_trait;
use eyre::Result;

pub mod terminal;

use makeup_console::Keypress;
pub use terminal::TerminalInput;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputFrame {
    Frame(Keypress),
    Empty,
    End,
}

#[async_trait]
pub trait Input: std::fmt::Debug + Send + Sync + Clone {
    async fn next_frame(&self) -> Result<InputFrame>;
}
