use async_trait::async_trait;
use eyre::Result;

use crate::input::InputFrame;
use crate::Input;

#[derive(Debug, Clone)]
pub struct TerminalInput {}

impl TerminalInput {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl Input for TerminalInput {
    async fn next_frame(&self) -> Result<InputFrame> {
        match makeup_console::next_keypress().await {
            Ok(key) => Ok(InputFrame::Frame(key)),
            Err(report) => {
                if let Some(err) = report.chain().next() {
                    match err.downcast_ref() {
                        Some(makeup_console::ConsoleError::Io(e)) => {
                            if e.kind() == std::io::ErrorKind::UnexpectedEof {
                                Ok(InputFrame::End)
                            } else {
                                Err(report)
                            }
                        }
                        Some(makeup_console::ConsoleError::Interrupted) => Ok(InputFrame::End),
                        None => Err(report),
                    }
                } else {
                    Err(report)
                }
            }
        }
    }
}
