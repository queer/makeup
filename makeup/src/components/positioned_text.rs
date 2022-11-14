use std::marker::PhantomData;

use async_trait::async_trait;
use eyre::Result;

use crate::component::{DrawCommandBatch, ExtractMessageFromComponent, Key, UpdateContext};
use crate::{Component, DrawCommand};

/// Simple component that renders text at the given (x, y).
#[derive(Debug)]
pub struct PositionedText<Message: std::fmt::Debug + Send + Sync + Clone> {
    text: String,
    x: usize,
    y: usize,
    key: Key,
    _phantom: PhantomData<Message>,
}

impl<Message: std::fmt::Debug + Send + Sync + Clone> PositionedText<Message> {
    pub fn new<S: Into<String>>(text: S, x: usize, y: usize) -> Self {
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

    async fn update(
        &mut self,
        _ctx: &mut UpdateContext<ExtractMessageFromComponent<Self>>,
    ) -> Result<()> {
        Ok(())
    }

    async fn render(&self) -> Result<DrawCommandBatch> {
        Ok((
            self.key,
            vec![DrawCommand::TextAt {
                text: self.text.clone(),
                x: self.x,
                y: self.y,
            }],
        ))
    }

    async fn update_pass(
        &mut self,
        _ctx: &mut UpdateContext<ExtractMessageFromComponent<Self>>,
    ) -> Result<()> {
        Ok(())
    }

    async fn render_pass(&self) -> Result<Vec<DrawCommandBatch>> {
        Ok(vec![self.render().await?])
    }

    fn key(&self) -> Key {
        self.key
    }
}

#[cfg(test)]
mod tests {
    use crate::components::PositionedText;
    use crate::render::MemoryRenderer;
    use crate::{Renderer, MUI};

    use eyre::Result;

    #[tokio::test]
    async fn test_it_works() -> Result<()> {
        let mut root = PositionedText::new("henol world", 1, 1);

        let mut renderer = MemoryRenderer::new(128, 128);
        let ui = MUI::<&'static str>::new(&mut root, &mut renderer);
        ui.render_frame().await?;

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
