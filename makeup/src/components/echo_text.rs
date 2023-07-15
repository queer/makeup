use std::marker::PhantomData;

use async_trait::async_trait;
use either::Either;
use eyre::Result;

use crate::component::{
    DrawCommandBatch, ExtractMessageFromComponent, Key, MakeupMessage, RenderContext, UpdateContext,
};
use crate::{check_mail, Component, DrawCommand};

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

    fn children(&self) -> Option<Vec<&dyn Component<Message = Self::Message>>> {
        None
    }

    async fn update(
        &mut self,
        ctx: &mut UpdateContext<ExtractMessageFromComponent<Self>>,
    ) -> Result<()> {
        check_mail!(
            self,
            ctx,
            match _ {
                MakeupMessage::TextUpdate(text) => {
                    self.text = text.clone();
                }
            }
        );

        Ok(())
    }

    async fn render(&self, _ctx: &RenderContext) -> Result<DrawCommandBatch> {
        self.batch(vec![DrawCommand::TextUnderCursor(self.text.clone())])
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
