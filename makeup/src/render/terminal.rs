use std::hash::{Hash, Hasher};
use std::io::Write;

use async_trait::async_trait;
use eyre::Result;

use crate::component::DrawCommandBatch;
use crate::{Ansi, DrawCommand};

use super::{MemoryRenderer, Renderer};

/// A [`Renderer`] that renders to a terminal.
#[derive(Debug)]
pub struct TerminalRenderer {
    memory_renderer: MemoryRenderer,
    saved_position: bool,
    hasher: Fnv,
    last_render_hash: Option<u64>,
}

struct Fnv(fnv::FnvHasher);

impl Fnv {
    pub fn reset(&mut self) {
        self.0 = fnv::FnvHasher::default();
    }
}

impl std::fmt::Debug for Fnv {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Fnv").field(&self.0.finish()).finish()
    }
}

impl TerminalRenderer {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        let (w, h) = ioctls::get_terminal_size();

        Self {
            memory_renderer: MemoryRenderer::new(w, h),
            saved_position: false,
            hasher: Fnv(fnv::FnvHasher::default()),
            last_render_hash: None,
        }
    }
}

#[async_trait]
impl Renderer for TerminalRenderer {
    async fn render(&mut self, commands: &[DrawCommandBatch]) -> Result<()> {
        // Save the cursor position before each render, and restore it after.
        // Not restoring the cursor position until we've saved it the first
        // time ensures that ex. the cursor will be positioned at the expected
        // character when rendering.
        if self.saved_position {
            print!("{}", Ansi::RestoreCursorPosition);

            // If the previous batch renders to the same hash as the current
            // batch, skip rendering the batch.
            self.hasher.reset();
            for command in commands {
                command.hash(&mut self.hasher.0);
            }
            let hash = self.hasher.0.finish();
            if Some(hash) == self.last_render_hash {
                return Ok(());
            }
        } else {
            self.saved_position = true;
        }
        print!("{}", Ansi::SaveCursorPosition);

        for (_key, commands) in commands {
            // debug!("rendering to terminal: {}", key);
            for command in commands {
                match command {
                    DrawCommand::TextUnderCursor(text) => {
                        print!("{}", text);
                    }
                    DrawCommand::CharUnderCursor(c) => {
                        print!("{}", c);
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

mod ioctls {
    pub fn get_terminal_size() -> (usize, usize) {
        use std::mem::zeroed;

        // Safety: Unfortuantely no other way to do this, ioctls suck.
        #[allow(unsafe_code)]
        unsafe {
            let mut size: libc::winsize = zeroed();
            // https://github.com/rust-lang/libc/pull/704
            // FIXME: ".into()" used as a temporary fix for a libc bug
            match libc::ioctl(libc::STDOUT_FILENO, libc::TIOCGWINSZ, &mut size) {
                0 => (size.ws_col as usize, size.ws_row as usize),
                _ => (80, 24),
            }
        }
    }
}
