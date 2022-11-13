use std::io::Write;

use async_trait::async_trait;
use eyre::Result;

use crate::component::DrawCommandBatch;
use crate::{Ansi, DrawCommand};

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
    async fn render(&mut self, commands: &[DrawCommandBatch]) -> Result<()> {
        self.memory_renderer.render(commands).await?;

        for (_key, commands) in commands {
            // debug!("rendering to terminal: {}", key);
            for command in commands {
                match command {
                    DrawCommand::TextUnderCursor(text) => {
                        print!("{}", text);
                    }
                    DrawCommand::TextAt { x, y, text } => {
                        print!("{}{}", Ansi::CursorPosition(*x, *y), text);
                    }
                    DrawCommand::MoveCursorRelative { x, y } => {
                        match x.cmp(&0) {
                            std::cmp::Ordering::Less => {
                                print!("{}", Ansi::CursorLeft(-x as usize));
                            }
                            std::cmp::Ordering::Equal => {}
                            std::cmp::Ordering::Greater => {
                                print!("{}", Ansi::CursorRight(*x as usize));
                            }
                        }

                        match y.cmp(&0) {
                            std::cmp::Ordering::Less => {
                                print!("{}", Ansi::CursorUp(-y as usize));
                            }
                            std::cmp::Ordering::Equal => {}
                            std::cmp::Ordering::Greater => {
                                print!("{}", Ansi::CursorDown(*y as usize));
                            }
                        }
                    }
                    DrawCommand::MoveCursorAbsolute { x, y } => {
                        print!("{}", Ansi::CursorPosition(*x, *y));
                    }
                }
            }
        }

        // NOTE: Can't flush with tokio, doesn't work for some reason.
        std::io::stdout().flush()?;

        Ok(())
    }

    async fn move_cursor(&mut self, x: usize, y: usize) -> eyre::Result<()> {
        let res = self.memory_renderer.move_cursor(x, y).await;
        print!("{}", Ansi::CursorPosition(x, y),);
        res
    }

    async fn move_cursor_relative(&mut self, x: isize, y: isize) -> eyre::Result<()> {
        let res = self.memory_renderer.move_cursor_relative(x, y).await;
        match x.cmp(&0) {
            std::cmp::Ordering::Less => {
                print!("{}", Ansi::CursorLeft(-x as usize));
            }
            std::cmp::Ordering::Equal => {}
            std::cmp::Ordering::Greater => {
                print!("{}", Ansi::CursorRight(x as usize));
            }
        }

        match y.cmp(&0) {
            std::cmp::Ordering::Less => {
                print!("{}", Ansi::CursorUp(-y as usize));
            }
            std::cmp::Ordering::Equal => {}
            std::cmp::Ordering::Greater => {
                print!("{}", Ansi::CursorDown(y as usize));
            }
        }
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
