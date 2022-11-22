use std::marker::PhantomData;

use async_trait::async_trait;
use either::Either;
use eyre::Result;
use makeup_ansi::LineEraseMode;
use makeup_console::Keypress;

use crate::component::{
    DrawCommandBatch, ExtractMessageFromComponent, Key, MakeupMessage, RenderContext, UpdateContext,
};
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

    fn children(&self) -> Option<Vec<&dyn Component<Message = Self::Message>>> {
        None
    }

    async fn update(
        &mut self,
        ctx: &mut UpdateContext<ExtractMessageFromComponent<Self>>,
    ) -> Result<()> {
        let mut offset = 0i32;
        check_mail!(self, ctx, {
            msg if MakeupMessage => {
                match msg {
                    MakeupMessage::Keypress(Keypress::Char(c)) => {
                        self.buffer.push(*c);
                    }
                    MakeupMessage::Keypress(Keypress::Backspace) => {
                        self.buffer.pop();
                        offset -= 1;
                    }
                    _ => {},
                }
            }
        });
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

    async fn update_pass(
        &mut self,
        ctx: &mut UpdateContext<ExtractMessageFromComponent<Self>>,
    ) -> Result<()> {
        self.update(ctx).await
    }

    async fn render_pass(&self, ctx: &RenderContext) -> Result<Vec<DrawCommandBatch>> {
        Ok(vec![self.render(ctx).await?])
    }

    fn key(&self) -> Key {
        self.key
    }
}

#[cfg(test)]
mod tests {
    use super::TextInput;
    use crate::component::{MessageSender, UpdateContext};
    use crate::post_office::PostOffice;
    use crate::{Component, DrawCommand};

    use eyre::Result;
    use makeup_console::Keypress;

    #[tokio::test]
    async fn test_it_works() -> Result<()> {
        let mut root = TextInput::<()>::new("henol world");
        let mut post_office = PostOffice::<()>::new();

        let (_k, render) = root.render(&crate::fake_render_ctx()).await?;
        assert_eq!(
            vec![
                DrawCommand::TextUnderCursor("henol world".into()),
                DrawCommand::CharUnderCursor(':'),
                DrawCommand::CharUnderCursor(' '),
                DrawCommand::TextUnderCursor("".into())
            ]
            .as_slice(),
            render.as_slice(),
        );

        post_office.send_makeup(
            root.key(),
            crate::component::MakeupMessage::Keypress(Keypress::Char('a')),
        );

        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        root.update_pass(&mut UpdateContext {
            post_office: &mut post_office,
            sender: MessageSender::new(tx.clone(), root.key()),
            focus: root.key(),
        })
        .await?;

        let (_k, render) = root.render(&crate::fake_render_ctx()).await?;
        assert_eq!(
            vec![
                DrawCommand::TextUnderCursor("henol world".into()),
                DrawCommand::CharUnderCursor(':'),
                DrawCommand::CharUnderCursor(' '),
                DrawCommand::TextUnderCursor("a".into())
            ]
            .as_slice(),
            render.as_slice(),
        );

        Ok(())
    }
}
