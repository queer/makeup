use std::marker::PhantomData;

use async_trait::async_trait;
use either::Either;
use eyre::Result;
use makeup_console::Keypress;

use crate::component::{
    DrawCommandBatch, ExtractMessageFromComponent, Key, MakeupMessage, RenderContext, UpdateContext,
};
use crate::{Component, DrawCommand};

/// A simple component that renders text under the cursor.
#[derive(Debug)]
pub struct TextInput<Message: std::fmt::Debug + Send + Sync + Clone> {
    prompt: String,
    key: Key,
    buffer: String,
    _phantom: PhantomData<Message>,
}

impl<Message: std::fmt::Debug + Send + Sync + Clone> TextInput<Message> {
    pub fn new<S: Into<String>>(prompt: S) -> Self {
        Self {
            prompt: prompt.into(),
            buffer: String::new(),
            key: crate::component::generate_key(),
            _phantom: PhantomData,
        }
    }
}

#[async_trait]
impl<Message: std::fmt::Debug + Send + Sync + Clone> Component for TextInput<Message> {
    type Message = Message;

    async fn update(
        &mut self,
        ctx: &mut UpdateContext<ExtractMessageFromComponent<Self>>,
    ) -> Result<()> {
        if let Some(mailbox) = ctx.post_office.mailbox(self) {
            for msg in mailbox.iter() {
                match msg {
                    Either::Left(_msg) => {
                        // log::debug!("Spinner received message: {:?}", msg);
                    }
                    #[allow(clippy::single_match)]
                    Either::Right(msg) => match msg {
                        // TODO: Handle arrow keys etc.
                        MakeupMessage::Keypress(Keypress::Char(c)) => {
                            self.buffer.push(*c);
                        }
                        // TODO: This has to handle the buffer state properly, because
                        MakeupMessage::Keypress(Keypress::Backspace) => {
                            self.buffer.pop();
                        }
                        _ => {}
                    },
                }
            }
            mailbox.clear();
        }

        Ok(())
    }

    async fn render(&self, _ctx: &RenderContext) -> Result<DrawCommandBatch> {
        Ok((
            self.key,
            vec![
                DrawCommand::TextUnderCursor(self.prompt.clone()),
                DrawCommand::CharUnderCursor(':'),
                DrawCommand::CharUnderCursor(' '),
                DrawCommand::TextUnderCursor(self.buffer.clone()),
            ],
        ))
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
    use crate::component::UpdateContext;
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
            tx: std::sync::Arc::new(tokio::sync::Mutex::new(tx)),
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
