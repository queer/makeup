use std::marker::PhantomData;

use async_trait::async_trait;
use eyre::Result;
use makeup_ansi::LineEraseMode;
use makeup_console::Keypress;

use crate::component::{DrawCommandBatch, Key, MakeupMessage, MakeupUpdate, RenderContext};
use crate::{check_mail, Component, DrawCommand};

/// A simple component that renders text under the cursor.
#[derive(Debug)]
pub struct TextInput<Message: std::fmt::Debug + Send + Sync + Clone> {
    prompt: String,
    key: Key,
    buffer: String,
    input_offset: Option<i32>,
    _phantom: PhantomData<Message>,
}

impl<Message: std::fmt::Debug + Send + Sync + Clone> TextInput<Message> {
    pub fn new<S: Into<String>>(prompt: S) -> Self {
        Self {
            prompt: prompt.into(),
            buffer: String::new(),
            key: crate::component::generate_key(),
            input_offset: None,
            _phantom: PhantomData,
        }
    }
}

#[async_trait]
impl<Message: std::fmt::Debug + Send + Sync + Clone> Component for TextInput<Message> {
    type Message = Message;

    fn children(&self) -> Option<Vec<&Box<dyn Component<Message = Self::Message>>>> {
        None
    }

    fn children_mut(&mut self) -> Option<Vec<&mut Box<dyn Component<Message = Self::Message>>>> {
        None
    }

    async fn update(&mut self, ctx: &mut MakeupUpdate<Self>) -> Result<()> {
        let mut offset = 0i32;
        check_mail!(
            self,
            ctx,
            match _ {
                MakeupMessage::Keypress(Keypress::Char(c)) => {
                    self.buffer.push(*c);
                }
                MakeupMessage::Keypress(Keypress::Backspace) => {
                    self.buffer.pop();
                    offset -= 1;
                }
            }
        );
        if offset != 0 {
            self.input_offset = Some(offset);
        } else {
            self.input_offset = None;
        }

        Ok(())
    }

    async fn render(&self, _ctx: &RenderContext) -> Result<DrawCommandBatch> {
        match self.input_offset {
            Some(offset) if offset < 0 => {
                // If we have a negative offset, erase to the end of the line.
                self.batch(vec![
                    DrawCommand::TextUnderCursor(self.prompt.clone()),
                    DrawCommand::CharUnderCursor(':'),
                    DrawCommand::CharUnderCursor(' '),
                    DrawCommand::TextUnderCursor(self.buffer.clone()),
                    // TODO: This should probably just replace the characters with whitespace...
                    DrawCommand::EraseCurrentLine(LineEraseMode::FromCursorToEnd),
                ])
            }
            _ => self.batch(vec![
                DrawCommand::TextUnderCursor(self.prompt.clone()),
                DrawCommand::CharUnderCursor(':'),
                DrawCommand::CharUnderCursor(' '),
                DrawCommand::TextUnderCursor(self.buffer.clone()),
            ]),
        }
    }

    fn key(&self) -> Key {
        self.key
    }

    fn dimensions(&self) -> Result<(u64, u64)> {
        // +2 comes from the `: ` between the prompt and the buffer.
        Ok((self.prompt.len() as u64 + 2 + self.buffer.len() as u64, 1))
    }
}

#[cfg(test)]
mod tests {
    use super::TextInput;
    use crate::component::{MessageSender, UpdateContext};
    use crate::post_office::PostOffice;
    use crate::test::assert_renders_many;
    use crate::{Component, DrawCommand};

    use eyre::Result;
    use makeup_console::Keypress;

    #[tokio::test]
    async fn test_it_works() -> Result<()> {
        let mut root = TextInput::<()>::new("henol world");
        let mut post_office = PostOffice::<()>::new();

        assert_renders_many!(
            vec![
                DrawCommand::TextUnderCursor("henol world".into()),
                DrawCommand::CharUnderCursor(':'),
                DrawCommand::CharUnderCursor(' '),
                DrawCommand::TextUnderCursor("".into())
            ],
            &root
        );

        post_office.send_makeup(
            root.key(),
            crate::component::MakeupMessage::Keypress(Keypress::Char('a')),
        );

        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        root.update(&mut UpdateContext {
            post_office: &mut post_office,
            sender: MessageSender::new(tx.clone(), root.key()),
            focus: root.key(),
            dimensions: (100, 100),
        })
        .await?;

        assert_renders_many!(
            vec![
                DrawCommand::TextUnderCursor("henol world".into()),
                DrawCommand::CharUnderCursor(':'),
                DrawCommand::CharUnderCursor(' '),
                DrawCommand::TextUnderCursor("a".into())
            ],
            &root
        );

        Ok(())
    }
}
