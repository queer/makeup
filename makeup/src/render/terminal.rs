use ansi_escapes::*;
use async_trait::async_trait;
use eyre::Result;

use crate::DrawCommand;

use super::{MemoryRenderer, Renderer};

#[derive(Debug)]
pub struct TerminalRenderer {
    memory_renderer: MemoryRenderer,
}

impl TerminalRenderer {
    pub fn new(width: usize, height: usize) -> Self {
        // TODO: Read terminal size
        Self {
            memory_renderer: MemoryRenderer::new(width, height),
        }
    }
}

#[async_trait]
impl Renderer for TerminalRenderer {
    async fn render(&mut self, commands: &[DrawCommand]) -> Result<()> {
        self.memory_renderer.render(commands).await?;

        for command in commands {
            match command {
                DrawCommand::TextUnderCursor(text) => {
                    print!("{}", text);
                }
                DrawCommand::TextAt { x, y, text } => {
                    print!("{}{}", CursorTo::AbsoluteXY(*y as u16, *x as u16), text);
                }
            }
        }

        Ok(())
    }

    async fn move_cursor(&mut self, x: usize, y: usize) -> eyre::Result<()> {
        self.memory_renderer.move_cursor(x, y).await
    }

    async fn read_at_cursor(&self, width: usize) -> eyre::Result<String> {
        self.memory_renderer.read_at_cursor(width).await
    }

    async fn read_string(&self, x: usize, y: usize, width: usize) -> eyre::Result<String> {
        self.memory_renderer.read_string(x, y, width).await
    }
}
