use async_trait::async_trait;
use eyre::Result;
use makeup_ansi::LineEraseMode;

use super::RenderError;
use crate::component::DrawCommandBatch;
use crate::{Coordinate, Coordinates, Dimension, Dimensions, RelativeCoordinate};
use crate::{DrawCommand, Renderer};

/// A [`Renderer`] that renders to an in-memory grid.
#[derive(Debug)]
pub struct MemoryRenderer {
    cursor_x: Coordinate,
    cursor_y: Coordinate,
    pub(crate) width: Dimension,
    pub(crate) height: Dimension,
    text: std::collections::HashMap<Coordinates, char>,
}

#[async_trait]
impl Renderer for MemoryRenderer {
    async fn render(&mut self, commands: &[DrawCommandBatch]) -> Result<()> {
        for (_key, commands) in commands {
            // debug!("rendering to terminal: {}", key);
            for command in commands {
                match command {
                    DrawCommand::TextUnderCursor(text) => {
                        let text_len = text.len() as Dimension;
                        self.bounds_check(self.cursor_x, self.cursor_y)?;
                        self.bounds_check(self.cursor_x + text_len, self.cursor_y)?;
                        for (i, c) in text.chars().enumerate() {
                            self.text
                                .insert((self.cursor_x + (i as Dimension), self.cursor_y), c);
                        }
                        // TODO: Newline detection
                        self.cursor_x += text_len;
                    }
                    DrawCommand::CharUnderCursor(c) => {
                        self.bounds_check(self.cursor_x, self.cursor_y)?;
                        self.bounds_check(self.cursor_x + 1, self.cursor_y)?;
                        self.text.insert((self.cursor_x, self.cursor_y), *c);
                        if *c == '\n' {
                            self.cursor_x = 0;
                            self.cursor_y += 1;
                        } else {
                            self.cursor_x += 1;
                        }
                    }
                    DrawCommand::EraseCurrentLine(mode) => match mode {
                        LineEraseMode::FromCursorToStart => {
                            for x in 0..self.cursor_x {
                                self.text.remove(&(x, self.cursor_y));
                            }
                        }
                        LineEraseMode::FromCursorToEnd => {
                            for x in self.cursor_x..self.width {
                                self.text.remove(&(x, self.cursor_y));
                            }
                        }
                        LineEraseMode::All => {
                            for x in 0..self.width {
                                self.text.remove(&(x, self.cursor_y));
                            }
                        }
                    },
                    DrawCommand::TextAt { x, y, text } => {
                        let text_len = text.len() as Dimension;
                        self.bounds_check(*x, *y)?;
                        self.bounds_check(*x + text_len, *y)?;
                        for (i, c) in text.chars().enumerate() {
                            self.text.insert((x + (i as Dimension), *y), c);
                        }
                        self.cursor_x = x + text_len;
                        self.cursor_y = *y;
                    }
                    DrawCommand::MoveCursorRelative { x, y } => {
                        let cursor_x = self.cursor_x as RelativeCoordinate;
                        let cursor_y = self.cursor_y as RelativeCoordinate;

                        self.bounds_check_relative(cursor_x + x, cursor_y + y)?;
                        self.cursor_x = (cursor_x + x) as Coordinate;
                        self.cursor_y = (cursor_y + y) as Coordinate;
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

    async fn flush(&mut self) -> Result<()> {
        Ok(())
    }

    async fn move_cursor(&mut self, x: Coordinate, y: Coordinate) -> Result<()> {
        self.bounds_check(x, y)?;
        self.cursor_x = x;
        self.cursor_y = y;
        Ok(())
    }

    async fn move_cursor_relative(
        &mut self,
        x: RelativeCoordinate,
        y: RelativeCoordinate,
    ) -> Result<()> {
        let cursor_x = self.cursor_x as RelativeCoordinate;
        let cursor_y = self.cursor_y as RelativeCoordinate;

        self.bounds_check_relative(cursor_x + x, cursor_y + y)?;
        self.cursor_x = (cursor_x + x) as Coordinate;
        self.cursor_y = (cursor_y + y) as Coordinate;
        Ok(())
    }

    async fn read_at_cursor(&self, width: Dimension) -> Result<String> {
        self.read_string(self.cursor_x, self.cursor_y, width).await
    }

    async fn read_string(&self, x: Coordinate, y: Coordinate, width: Dimension) -> Result<String> {
        self.bounds_check(x, y)?;
        self.bounds_check(x + width, y)?;
        let mut result = String::new();
        for i in 0..width {
            result.push(*self.text.get(&(x + i, y)).unwrap_or(&' '));
        }
        Ok(result)
    }

    fn cursor(&self) -> Coordinates {
        (self.cursor_x, self.cursor_y)
    }

    fn dimensions(&self) -> Dimensions {
        (self.width, self.height)
    }

    fn set_width(&mut self, width: Dimension) {
        self.width = width;
    }

    fn set_height(&mut self, height: Dimension) {
        self.height = height;
    }
}

impl MemoryRenderer {
    pub fn new(width: Dimension, height: Dimension) -> MemoryRenderer {
        MemoryRenderer {
            cursor_x: 0,
            cursor_y: 0,
            width,
            height,
            text: std::collections::HashMap::new(),
        }
    }

    // TODO: Should we just be truncating instead?
    fn bounds_check(&self, x: Coordinate, y: Coordinate) -> Result<()> {
        if x < self.width && y < self.height {
            Ok(())
        } else {
            Err(RenderError::OutOfBounds(x as RelativeCoordinate, y as RelativeCoordinate).into())
        }
    }

    fn bounds_check_relative(&self, x: RelativeCoordinate, y: RelativeCoordinate) -> Result<()> {
        if x < self.width as RelativeCoordinate
            && y < self.height as RelativeCoordinate
            && x >= 0
            && y >= 0
        {
            Ok(())
        } else {
            Err(RenderError::OutOfBounds(x, y).into())
        }
    }
}
