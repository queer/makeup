use async_trait::async_trait;
use eyre::Result;
use makeup_console::ConsoleState;

use crate::input::InputFrame;
use crate::Input;

#[derive(Debug, Clone)]
pub struct TerminalInput {
    state: ConsoleState<'static>,
}

impl TerminalInput {
    pub async fn new() -> Result<Self> {
        Ok(Self {
            state: makeup_console::init().await?,
        })
    }
}

#[async_trait]
impl Input for TerminalInput {
    async fn next_frame(&self) -> Result<InputFrame> {
        match makeup_console::next_keypress(&self.state).await {
            Ok(Some(key)) => Ok(InputFrame::Frame(key)),
            Ok(_) => Ok(InputFrame::Empty),
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
