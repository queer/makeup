use async_trait::async_trait;
use eyre::Result;

use super::RenderError;
use crate::{component::DrawCommandBatch, DrawCommand, Renderer};

#[derive(Debug)]
pub struct MemoryRenderer {
    cursor_x: usize,
    cursor_y: usize,
    width: usize,
    height: usize,
    text: std::collections::HashMap<(usize, usize), char>,
}

#[async_trait]
impl Renderer for MemoryRenderer {
    async fn render(&mut self, commands: &[DrawCommandBatch]) -> Result<()> {
        for (_key, commands) in commands {
            // debug!("rendering to terminal: {}", key);
            for command in commands {
                match command {
                    DrawCommand::TextUnderCursor(text) => {
                        self.bounds_check(self.cursor_x, self.cursor_y)?;
                        self.bounds_check(self.cursor_x + text.len(), self.cursor_y)?;
                        for (i, c) in text.chars().enumerate() {
                            self.text.insert((self.cursor_x + i, self.cursor_y), c);
                        }
                        self.cursor_x += text.len();
                    }
                    DrawCommand::TextAt { x, y, text } => {
                        self.bounds_check(*x, *y)?;
                        self.bounds_check(*x + text.len(), *y)?;
                        for (i, c) in text.chars().enumerate() {
                            self.text.insert((x + i, *y), c);
                        }
                        self.cursor_x = x + text.len();
                        self.cursor_y = *y;
                    }
                    DrawCommand::MoveCursorRelative { x, y } => {
                        let cursor_x = self.cursor_x as isize;
                        let cursor_y = self.cursor_y as isize;

                        self.bounds_check_relative(cursor_x + x, cursor_y + y)?;
                        self.cursor_x = (cursor_x + *x) as usize;
                        self.cursor_y = (cursor_y + *y) as usize;
                    }
                    DrawCommand::MoveCursorAbsolute { x, y } => {
                        self.bounds_check(*x, *y)?;
                        self.cursor_x = *x;
                        self.cursor_y = *y;
                    }
                }
            }
        }
        Ok(())
    }

    async fn move_cursor(&mut self, x: usize, y: usize) -> Result<()> {
        self.bounds_check(x, y)?;
        self.cursor_x = x;
        self.cursor_y = y;
        Ok(())
    }

    async fn move_cursor_relative(&mut self, x: isize, y: isize) -> Result<()> {
        let cursor_x = self.cursor_x as isize;
        let cursor_y = self.cursor_y as isize;

        self.bounds_check_relative(cursor_x + x, cursor_y + y)?;
        self.cursor_x = (cursor_x + x) as usize;
        self.cursor_y = (cursor_y + y) as usize;
        Ok(())
    }

    async fn read_at_cursor(&self, width: usize) -> Result<String> {
        self.read_string(self.cursor_x, self.cursor_y, width).await
    }

    async fn read_string(&self, x: usize, y: usize, width: usize) -> Result<String> {
        self.bounds_check(x, y)?;
        self.bounds_check(x + width, y)?;
        let mut result = String::new();
        for i in 0..width {
            result.push(*self.text.get(&(x + i, y)).unwrap_or(&' '));
        }
        Ok(result)
    }

    fn cursor(&self) -> (usize, usize) {
        (self.cursor_x, self.cursor_y)
    }
}

impl MemoryRenderer {
    pub fn new(width: usize, height: usize) -> MemoryRenderer {
        MemoryRenderer {
            cursor_x: 0,
            cursor_y: 0,
            width,
            height,
            text: std::collections::HashMap::new(),
        }
    }

    // TODO: Should we just be truncating instead?
    fn bounds_check(&self, x: usize, y: usize) -> Result<()> {
        if x < self.width && y < self.height {
            Ok(())
        } else {
            Err(RenderError::OutOfBounds(x as isize, y as isize).into())
        }
    }

    fn bounds_check_relative(&self, x: isize, y: isize) -> Result<()> {
        if x < self.width as isize && y < self.height as isize && x >= 0 && y >= 0 {
            Ok(())
        } else {
            Err(RenderError::OutOfBounds(x, y).into())
        }
    }
}
