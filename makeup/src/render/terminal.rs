use std::io::Write;

use async_trait::async_trait;
use eyre::Result;
use makeup_ansi::{CursorVisibility, DisplayEraseMode, SgrParameter};

use crate::component::DrawCommandBatch;
use crate::{Ansi, DrawCommand, DrawStyle};
use crate::{Coordinate, Coordinates, Dimension, RelativeCoordinate};

use super::{MemoryRenderer, Renderer};

/// A [`Renderer`] that renders to a terminal.
#[derive(Debug)]
pub struct TerminalRenderer {
    memory_renderer: MemoryRenderer,
    saved_position: bool,
}

impl TerminalRenderer {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        let (w, h) = ioctls::get_terminal_size();

        Self {
            memory_renderer: MemoryRenderer::new(w, h),
            saved_position: false,
        }
    }
}

#[async_trait]
impl Renderer for TerminalRenderer {
    async fn render(&mut self, commands: &[DrawCommandBatch]) -> Result<()> {
        let mut buffer = String::new();

        // Handle terminal resizes.
        // Blank the screen to avoid lingering state on rerender.
        let (w, h) = ioctls::get_terminal_size();
        if w != self.memory_renderer.width || h != self.memory_renderer.height {
            self.set_width(w);
            self.set_height(h);
            buffer += &Ansi::EraseInDisplay(DisplayEraseMode::All).to_string();
        }

        // Save the cursor position before each render, and restore it after.
        // Not restoring the cursor position until we've saved it the first
        // time ensures that ex. the cursor will be positioned at the expected
        // character when rendering.
        if self.saved_position {
            buffer += &Ansi::RestoreCursorPosition.to_string();
        } else {
            self.saved_position = true;
        }
        buffer += &Ansi::SaveCursorPosition.to_string();

        for (_key, commands) in commands {
            // debug!("rendering to terminal: {}", key);
            for command in commands {
                match command {
                    DrawCommand::TextUnderCursor(text) => {
                        buffer += text;
                    }

                    DrawCommand::CharUnderCursor(c) => {
                        buffer.push(*c);
                    }

                    DrawCommand::EraseCurrentLine(mode) => {
                        buffer += &Ansi::EraseInLine(mode.clone()).to_string();
                    }

                    DrawCommand::MoveCursorRelative { x, y } => {
                        match x.cmp(&0) {
                            std::cmp::Ordering::Less => {
                                buffer += &Ansi::CursorLeft(-x as Dimension).to_string();
                            }
                            std::cmp::Ordering::Equal => {}
                            std::cmp::Ordering::Greater => {
                                buffer += &Ansi::CursorRight(*x as Dimension).to_string();
                            }
                        }

                        match y.cmp(&0) {
                            std::cmp::Ordering::Less => {
                                buffer += &Ansi::CursorUp(-y as Dimension).to_string();
                            }
                            std::cmp::Ordering::Equal => {}
                            std::cmp::Ordering::Greater => {
                                buffer += &Ansi::CursorDown(*y as Dimension).to_string();
                            }
                        }
                    }

                    DrawCommand::MoveCursorAbsolute { x, y } => {
                        buffer += &Ansi::CursorPosition(*x, *y).to_string();
                    }

                    DrawCommand::HideCursor => {
                        buffer += &Ansi::CursorVisibility(CursorVisibility::Invisible).to_string();
                    }

                    DrawCommand::ShowCursor => {
                        buffer += &Ansi::CursorVisibility(CursorVisibility::Visible).to_string();
                    }

                    DrawCommand::Style(style) => match style {
                        DrawStyle::Coloured {
                            foreground,
                            background,
                        } => {
                            if let Some(background) = background {
                                buffer += &Ansi::Sgr(vec![SgrParameter::HexBackgroundColour(
                                    *background,
                                )])
                                .to_string();
                            }
                            if let Some(foreground) = foreground {
                                buffer += &Ansi::Sgr(vec![SgrParameter::HexForegroundColour(
                                    *foreground,
                                )])
                                .to_string();
                            }
                        }

                        DrawStyle::Foreground(foreground) => {
                            buffer +=
                                &Ansi::Sgr(vec![SgrParameter::HexForegroundColour(*foreground)])
                                    .to_string();
                        }

                        DrawStyle::Background(background) => {
                            buffer +=
                                &Ansi::Sgr(vec![SgrParameter::HexBackgroundColour(*background)])
                                    .to_string();
                        }

                        DrawStyle::Coloured8Bit {
                            foreground,
                            background,
                        } => {
                            if let Some(background) = background {
                                buffer += &Ansi::TerminalBackgroundColour(*background).to_string();
                            }
                            if let Some(foreground) = foreground {
                                buffer += &Ansi::TerminalForegroundColour(*foreground).to_string();
                            }
                        }

                        DrawStyle::Foreground8Bit(foreground) => {
                            buffer += &Ansi::TerminalForegroundColour(*foreground).to_string();
                        }

                        DrawStyle::Background8Bit(background) => {
                            buffer += &Ansi::TerminalBackgroundColour(*background).to_string();
                        }

                        DrawStyle::Default => {
                            buffer += &Ansi::Sgr(vec![SgrParameter::Reset]).to_string();
                        }

                        DrawStyle::Bold => {
                            buffer += &Ansi::Sgr(vec![SgrParameter::Bold]).to_string();
                        }

                        DrawStyle::Italic => {
                            buffer += &Ansi::Sgr(vec![SgrParameter::Italic]).to_string();
                        }

                        DrawStyle::Underline => {
                            buffer += &Ansi::Sgr(vec![SgrParameter::Underline]).to_string();
                        }
                    },
                }
            }
        }

        print!("{buffer}");

        Ok(())
    }

    async fn flush(&mut self) -> Result<()> {
        // NOTE: Can't flush with tokio, doesn't work for some reason.
        std::io::stdout().flush()?;
        Ok(())
    }

    async fn move_cursor(&mut self, x: Coordinate, y: Coordinate) -> eyre::Result<()> {
        let res = self.memory_renderer.move_cursor(x, y).await;
        print!("{}", Ansi::CursorPosition(x, y),);
        res
    }

    async fn move_cursor_relative(
        &mut self,
        x: RelativeCoordinate,
        y: RelativeCoordinate,
    ) -> eyre::Result<()> {
        let res = self.memory_renderer.move_cursor_relative(x, y).await;
        match x.cmp(&0) {
            std::cmp::Ordering::Less => {
                print!("{}", Ansi::CursorLeft(-x as Dimension));
            }
            std::cmp::Ordering::Equal => {}
            std::cmp::Ordering::Greater => {
                print!("{}", Ansi::CursorRight(x as Dimension));
            }
        }

        match y.cmp(&0) {
            std::cmp::Ordering::Less => {
                print!("{}", Ansi::CursorUp(-y as Dimension));
            }
            std::cmp::Ordering::Equal => {}
            std::cmp::Ordering::Greater => {
                print!("{}", Ansi::CursorDown(y as Dimension));
            }
        }
        res
    }

    async fn read_at_cursor(&self, width: Dimension) -> eyre::Result<String> {
        self.memory_renderer.read_at_cursor(width).await
    }

    async fn read_string(
        &self,
        x: Coordinate,
        y: Coordinate,
        width: Dimension,
    ) -> eyre::Result<String> {
        self.memory_renderer.read_string(x, y, width).await
    }

    fn cursor(&self) -> Coordinates {
        self.memory_renderer.cursor()
    }

    fn dimensions(&self) -> Coordinates {
        self.memory_renderer.dimensions()
    }

    fn set_width(&mut self, width: Dimension) {
        self.memory_renderer.set_width(width);
    }

    fn set_height(&mut self, height: Dimension) {
        self.memory_renderer.set_height(height);
    }
}

mod ioctls {
    use crate::{Dimension, Dimensions};

    pub fn get_terminal_size() -> Dimensions {
        use std::mem::zeroed;

        // Safety: Unfortuantely no other way to do this, ioctls suck.
        #[allow(unsafe_code)]
        unsafe {
            let mut size: libc::winsize = zeroed();
            match libc::ioctl(libc::STDOUT_FILENO, libc::TIOCGWINSZ, &mut size) {
                0 => (size.ws_col as Dimension, size.ws_row as Dimension),
                _ => (80, 24),
            }
        }
    }
}
