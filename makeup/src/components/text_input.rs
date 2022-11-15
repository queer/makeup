use std::marker::PhantomData;

use async_trait::async_trait;
use either::Either;
use eyre::Result;

use crate::component::{
    DrawCommandBatch, ExtractMessageFromComponent, Key, MakeupMessage, RenderContext, UpdateContext,
};
use crate::{Component, DrawCommand};

/// A simple component that renders text under the cursor.
#[derive(Debug)]
pub struct TextInput<Message: std::fmt::Debug + Send + Sync + Clone> {
    prompt: String,
    key: Key,
    _phantom: PhantomData<Message>,
}

impl<Message: std::fmt::Debug + Send + Sync + Clone> TextInput<Message> {
    pub fn new<S: Into<String>>(prompt: S) -> Self {
        Self {
            prompt: prompt.into(),
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
                        MakeupMessage::TextUpdate(text) => {
                            self.text = text.clone();
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
            vec![DrawCommand::TextUnderCursor(self.text.clone())],
        ))
    }

    async fn update_pass(
        &mut self,
        _ctx: &mut UpdateContext<ExtractMessageFromComponent<Self>>,
    ) -> Result<()> {
        Ok(())
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
    use crate::{Component, DrawCommand};

    use eyre::Result;

    #[tokio::test]
    async fn test_it_works() -> Result<()> {
        let root = TextInput::<()>::new("henol world");

        let (_k, render) = root.render(&crate::fake_render_ctx()).await?;
        assert_eq!(
            vec![DrawCommand::TextUnderCursor("henol world".to_string(),)].as_slice(),
            render.as_slice(),
        );

        Ok(())
    }
}
