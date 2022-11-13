use std::io::Write;

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
                    print!(
                        "{}{}",
                        CursorTo::AbsoluteXY(*y as u16, (*x + text.len()) as u16),
                        text
                    );
                }
                DrawCommand::MoveCursorRelative { x, y } => {
                    print!("{}", CursorMove::XY(*x as i16, *y as i16));
                }
                DrawCommand::MoveCursorAbsolute { x, y } => {
                    print!("{}", CursorTo::AbsoluteXY(*x as u16, *y as u16));
                }
            }
        }

        // NOTE: Can't flush with tokio, doesn't work for some reason.
        std::io::stdout().flush()?;

        Ok(())
    }

    async fn move_cursor(&mut self, x: usize, y: usize) -> eyre::Result<()> {
        let res = self.memory_renderer.move_cursor(x, y).await;
        print!("{}", CursorTo::AbsoluteXY(y as u16, x as u16));
        res
    }

    async fn move_cursor_relative(&mut self, x: isize, y: isize) -> eyre::Result<()> {
        let res = self.memory_renderer.move_cursor_relative(x, y).await;
        print!("{}", CursorMove::XY(x as i16, y as i16));
        res
    }

    async fn read_at_cursor(&self, width: usize) -> eyre::Result<String> {
        self.memory_renderer.read_at_cursor(width).await
    }

    async fn read_string(&self, x: usize, y: usize, width: usize) -> eyre::Result<String> {
        self.memory_renderer.read_string(x, y, width).await
    }

    fn cursor(&self) -> (usize, usize) {
        self.memory_renderer.cursor()
    }
}
