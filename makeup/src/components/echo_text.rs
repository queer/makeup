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
pub struct EchoText<Message: std::fmt::Debug + Send + Sync + Clone> {
    text: String,
    key: Key,
    _phantom: PhantomData<Message>,
}

impl<Message: std::fmt::Debug + Send + Sync + Clone> EchoText<Message> {
    pub fn new<S: Into<String>>(text: S) -> Self {
        Self {
            text: text.into(),
            key: crate::component::generate_key(),
            _phantom: PhantomData,
        }
    }
}

#[async_trait]
impl<Message: std::fmt::Debug + Send + Sync + Clone> Component for EchoText<Message> {
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
    use super::EchoText;
    use crate::{Component, DrawCommand};

    use eyre::Result;

    #[tokio::test]
    async fn test_it_works() -> Result<()> {
        let root = EchoText::<()>::new("henol world");

        let (_k, render) = root.render(&crate::fake_render_ctx()).await?;
        assert_eq!(
            vec![DrawCommand::TextUnderCursor("henol world".into(),)].as_slice(),
            render.as_slice(),
        );

        Ok(())
    }
}
