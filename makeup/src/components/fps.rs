use std::marker::PhantomData;

use async_trait::async_trait;
use eyre::Result;

use crate::component::{DrawCommandBatch, Key, MakeupUpdate, RenderContext};
use crate::{Component, Dimensions, DrawCommand};

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

    fn children(&self) -> Option<Vec<&Box<dyn Component<Message = Self::Message>>>> {
        None
    }

    fn children_mut(&mut self) -> Option<Vec<&mut Box<dyn Component<Message = Self::Message>>>> {
        None
    }

    async fn update(&mut self, _ctx: &mut MakeupUpdate<Self>) -> Result<()> {
        Ok(())
    }

    async fn render(&self, ctx: &RenderContext) -> Result<DrawCommandBatch> {
        self.batch(
            vec![DrawCommand::TextUnderCursor(format!(
                "FPS: {:.2} (effective: {: >10.2}), dimensions: ({}, {}), cursor (when this render started): ({}, {}), last frame: {}ms, frame: {}",
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

    fn key(&self) -> Key {
        self.key
    }

    fn dimensions(&self) -> Result<Option<Dimensions>> {
        // TODO: This should be the dimensions of the text, but we don't have that yet.
        Ok(Some((0, 0)))
    }
}

#[cfg(test)]
mod tests {
    use super::Fps;
    use crate::test::{assert_renders_one, static_text};

    use eyre::Result;

    #[tokio::test]
    async fn test_it_works() -> Result<()> {
        let mut root = Fps::<()>::new();
        assert_renders_one!(
            static_text!("FPS: 0.00 (effective:       0.00), dimensions: (0, 0), cursor (when this render started): (0, 0), last frame: 0ms, frame: 0"),
            root
        );

        Ok(())
    }
}
