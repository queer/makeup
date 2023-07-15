use std::marker::PhantomData;

use async_trait::async_trait;
use eyre::Result;

use crate::component::{
    DrawCommandBatch, ExtractMessageFromComponent, Key, RenderContext, UpdateContext,
};
use crate::{Component, DrawCommand};

/// A simple component that renders text under the cursor.
#[derive(Debug)]
pub struct Fps<Message: std::fmt::Debug + Send + Sync + Clone> {
    key: Key,
    _phantom: PhantomData<Message>,
}

impl<Message: std::fmt::Debug + Send + Sync + Clone> Fps<Message> {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            key: crate::component::generate_key(),
            _phantom: PhantomData,
        }
    }
}

#[async_trait]
impl<Message: std::fmt::Debug + Send + Sync + Clone> Component for Fps<Message> {
    type Message = Message;

    fn children(&self) -> Option<Vec<&dyn Component<Message = Self::Message>>> {
        None
    }

    async fn update(
        &mut self,
        _ctx: &mut UpdateContext<ExtractMessageFromComponent<Self>>,
    ) -> Result<()> {
        Ok(())
    }

    async fn render(&self, ctx: &RenderContext) -> Result<DrawCommandBatch> {
        self.batch(
            vec![DrawCommand::TextUnderCursor(format!(
                "FPS: {:.2} (effective: {:.2}), dimensions: ({}, {}), cursor (when this render started): ({}, {}), last frame: {}ms, frame: {}",
                ctx.fps,
                ctx.effective_fps,
                ctx.dimensions.0,
                ctx.dimensions.1,
                ctx.cursor.0,
                ctx.cursor.1,
                ctx.last_frame_time.map(|d| d.as_millis()).unwrap_or(0),
                ctx.frame_counter
            ))],
        )
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
    use super::Fps;
    use crate::{assert_renders_one, static_text, Component};

    use eyre::Result;

    #[tokio::test]
    async fn test_it_works() -> Result<()> {
        let root = Fps::<()>::new();
        assert_renders_one!(
            static_text!("FPS: 0.00 (effective: 0.00), dimensions: (0, 0), cursor (when this render started): (0, 0), last frame: 0ms, frame: 0"),
            root
        );

        Ok(())
    }
}
