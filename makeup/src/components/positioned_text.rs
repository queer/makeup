use std::marker::PhantomData;

use async_trait::async_trait;
use either::Either;
use eyre::Result;

use crate::component::{
    DrawCommandBatch, ExtractMessageFromComponent, Key, MakeupMessage, RenderContext, UpdateContext,
};
use crate::{check_mail, Component, Coordinate, DrawCommand};

/// Simple component that renders text at the given (x, y).
#[derive(Debug)]
pub struct PositionedText<Message: std::fmt::Debug + Send + Sync + Clone> {
    text: String,
    x: Coordinate,
    y: Coordinate,
    key: Key,
    _phantom: PhantomData<Message>,
}

impl<Message: std::fmt::Debug + Send + Sync + Clone> PositionedText<Message> {
    pub fn new<S: Into<String>>(text: S, x: Coordinate, y: Coordinate) -> Self {
        Self {
            text: text.into(),
            x,
            y,
            key: crate::component::generate_key(),
            _phantom: PhantomData,
        }
    }
}

#[async_trait]
impl<Message: std::fmt::Debug + Send + Sync + Clone> Component for PositionedText<Message> {
    type Message = Message;

    fn children(&self) -> Option<Vec<&dyn Component<Message = Self::Message>>> {
        None
    }

    async fn update(
        &mut self,
        ctx: &mut UpdateContext<ExtractMessageFromComponent<Self>>,
    ) -> Result<()> {
        check_mail!(self, ctx, {
            MakeupMessage = msg => {
                if let MakeupMessage::TextUpdate(text) = msg {
                    self.text = text.clone();
                }
            }
        });

        Ok(())
    }

    async fn render(&self, _ctx: &RenderContext) -> Result<DrawCommandBatch> {
        self.batch(vec![DrawCommand::TextAt {
            text: self.text.clone(),
            x: self.x,
            y: self.y,
        }])
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
    use crate::components::PositionedText;
    use crate::input::TerminalInput;
    use crate::render::MemoryRenderer;
    use crate::{Renderer, MUI};

    use eyre::Result;

    #[tokio::test]
    async fn test_it_works() -> Result<()> {
        let mut root = PositionedText::<()>::new("henol world", 1, 1);

        let mut renderer = MemoryRenderer::new(128, 128);
        let input = TerminalInput::new();
        let ui = MUI::new(&mut root, &mut renderer, input);
        ui.render_once().await?;

        renderer.move_cursor(0, 0).await?;
        assert_eq!(" ", renderer.read_at_cursor(1).await?);

        renderer.move_cursor(1, 1).await?;
        assert_eq!(
            "henol world".to_string(),
            renderer.read_at_cursor(11).await?
        );

        Ok(())
    }
}
