use async_trait::async_trait;
use eyre::Result;

use super::RenderError;
use crate::{DrawCommand, Renderer};

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
    async fn render(&mut self, commands: &[DrawCommand]) -> Result<()> {
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
            Err(RenderError::OutOfBounds(x, y).into())
        }
    }
}
