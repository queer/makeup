#![deny(unsafe_code)]

pub mod component;
pub mod components;
pub mod post_office;
pub mod render;
pub mod ui;
pub mod util;

pub use component::Component;
pub use render::Renderer;
pub use ui::MUI;

pub use makeup_ansi::prelude::*;

/// Commands for drawing to the character grid. Draw commands are processed by
/// the current [`Renderer`].
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DrawCommand {
    /// Draw text under the cursor, advancing the cursor by `text.len()`
    /// characters.
    TextUnderCursor(String),

    /// Draw text at the given (x, y), moving the cursor to
    /// `(x + text.len(), y)`.
    TextAt { x: usize, y: usize, text: String },

    /// Move the cursor relative to its current position.
    MoveCursorRelative { x: isize, y: isize },

    /// Move the cursor absolutely.
    MoveCursorAbsolute { x: usize, y: usize },
}

#[cfg(test)]
mod tests {
    use crate::component::{DrawCommandBatch, ExtractMessageFromComponent, Key, UpdateContext};
    use crate::components::EchoText;
    use crate::render::MemoryRenderer;
    use crate::util::RwLocked;
    use crate::{Component, DrawCommand, Renderer, MUI};

    use async_trait::async_trait;
    use eyre::Result;

    #[derive(Debug)]
    struct BasicComponent<'a> {
        #[allow(dead_code)]
        state: (),
        children: Vec<RwLocked<&'a mut dyn Component<Message = ()>>>,
        key: Key,
    }

    #[async_trait]
    impl<'a> Component for BasicComponent<'a> {
        type Message = ();

        async fn update(
            &mut self,
            _ctx: &mut UpdateContext<ExtractMessageFromComponent<Self>>,
        ) -> Result<()> {
            Ok(())
        }

        async fn render(&self) -> Result<DrawCommandBatch> {
            Ok((
                self.key,
                vec![DrawCommand::TextUnderCursor("henol world".to_string())],
            ))
        }

        async fn update_pass(
            &mut self,
            _ctx: &mut UpdateContext<ExtractMessageFromComponent<Self>>,
        ) -> Result<()> {
            Ok(())
        }

        async fn render_pass(&self) -> Result<Vec<DrawCommandBatch>> {
            let mut out = vec![];
            let render = self.render().await?;
            out.push(render);

            for child in &self.children {
                let child = child.read().await;
                let mut render = child.render_pass().await?;
                out.append(&mut render);
            }

            Ok(out)
        }

        fn key(&self) -> Key {
            self.key
        }
    }

    #[tokio::test]
    async fn test_it_works() -> Result<()> {
        let mut root = BasicComponent {
            state: (),
            children: vec![],
            key: crate::component::generate_key(),
        };

        let mut renderer = MemoryRenderer::new(128, 128);
        let ui = MUI::new(&mut root, &mut renderer);
        ui.render_frame().await?;
        let expected = "henol world".to_string();
        ui.render_frame().await?;
        renderer.move_cursor(0, 0).await?;
        assert_eq!(expected, renderer.read_at_cursor(expected.len()).await?);

        Ok(())
    }

    #[tokio::test]
    async fn test_it_renders_children() -> Result<()> {
        let mut child = EchoText::new("? wrong! banana!");

        let mut root = BasicComponent {
            state: (),
            children: vec![RwLocked::new(&mut child)],
            key: crate::component::generate_key(),
        };

        let mut renderer = MemoryRenderer::new(128, 128);
        let ui = MUI::new(&mut root, &mut renderer);
        ui.render_frame().await?;

        let expected = "henol world? wrong! banana".to_string();
        renderer.move_cursor(0, 0).await?;
        assert_eq!(expected, renderer.read_at_cursor(expected.len()).await?);

        Ok(())
    }
}
