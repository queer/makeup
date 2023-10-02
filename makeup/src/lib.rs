#![deny(unsafe_code)]
#![deny(
    rustdoc::broken_intra_doc_links,
    rustdoc::bare_urls,
    rustdoc::private_intra_doc_links
)]
#![deny(clippy::perf, clippy::complexity, clippy::cargo)]
#![allow(clippy::new_without_default)]
#![allow(clippy::multiple_crate_versions)]

pub mod component;
pub mod components;
pub mod input;
pub mod post_office;
pub mod render;
pub mod test;
pub mod ui;
pub mod util;

pub use component::Component;
pub use input::Input;
pub use render::Renderer;
pub use ui::MUI;

pub use makeup_ansi::prelude::*;

pub type Coordinate = u64;
pub type Coordinates = (Coordinate, Coordinate);
pub type Dimension = u64;
pub type Dimensions = (Dimension, Dimension);
pub type RelativeCoordinate = i64;

/// Commands for drawing to the character grid. Draw commands are processed by
/// the current [`Renderer`].
#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, strum::Display)]
pub enum DrawCommand {
    /// Draw text under the cursor, advancing the cursor by `text.len()`
    /// characters.
    TextUnderCursor(String),

    /// Draw a single character under the cursor, advancing the cursor by 1
    /// character.
    CharUnderCursor(char),

    /// Erase the current line, with behaviour depending on the position of the
    /// cursor and the [`LineEraseMode`] passed in.
    EraseCurrentLine(LineEraseMode),

    /// Draw text at the given (x, y), moving the cursor to
    /// `(x + text.len(), y)`.
    TextAt {
        x: Coordinate,
        y: Coordinate,
        text: String,
    },

    /// Move the cursor relative to its current position.
    MoveCursorRelative {
        x: RelativeCoordinate,
        y: RelativeCoordinate,
    },

    /// Move the cursor absolutely.
    MoveCursorAbsolute { x: Coordinate, y: Coordinate },

    /// Hide the cursor.
    HideCursor,

    /// Show the cursor.
    ShowCursor,
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::component::{
        DrawCommandBatch, ExtractMessageFromComponent, Key, RenderContext, UpdateContext,
    };
    use crate::components::EchoText;
    use crate::input::TerminalInput;
    use crate::render::MemoryRenderer;
    use crate::ui::RwLocked;
    use crate::{Component, DrawCommand, MUI};

    use async_trait::async_trait;
    use eyre::Result;
    use tokio::sync::RwLock;

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

        fn children(&self) -> Option<Vec<&dyn Component<Message = Self::Message>>> {
            None
        }

        async fn update(
            &mut self,
            _ctx: &mut UpdateContext<ExtractMessageFromComponent<Self>>,
        ) -> Result<()> {
            Ok(())
        }

        async fn render(&self, _ctx: &RenderContext) -> Result<DrawCommandBatch> {
            Ok((
                self.key,
                vec![DrawCommand::TextUnderCursor("henol world".into())],
            ))
        }

        async fn update_pass(
            &mut self,
            _ctx: &mut UpdateContext<ExtractMessageFromComponent<Self>>,
        ) -> Result<()> {
            Ok(())
        }

        async fn render_pass(&self, ctx: &RenderContext) -> Result<Vec<DrawCommandBatch>> {
            let mut out = vec![];
            let render = self.render(ctx).await?;
            out.push(render);

            for child in &self.children {
                let child = child.read().await;
                let mut render = child.render_pass(ctx).await?;
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

        let renderer = MemoryRenderer::new(128, 128);
        let input = TerminalInput::new().await?;
        let ui = MUI::new(&mut root, Box::new(renderer), input);
        ui.render_once().await?;
        let expected = "henol world".to_string();
        ui.render_once().await?;
        ui.move_cursor(0, 0).await?;
        assert_eq!(expected, ui.read_at_cursor(expected.len() as u64).await?);

        Ok(())
    }

    #[tokio::test]
    async fn test_it_renders_children() -> Result<()> {
        let mut child = EchoText::new("? wrong! banana!");

        let mut root = BasicComponent {
            state: (),
            children: vec![Arc::new(RwLock::new(&mut child))],
            key: crate::component::generate_key(),
        };

        let renderer = MemoryRenderer::new(128, 128);
        let input = TerminalInput::new().await?;
        let ui = MUI::new(&mut root, Box::new(renderer), input);
        ui.render_once().await?;

        let expected = "henol world? wrong! banana".to_string();
        ui.move_cursor(0, 0).await?;
        assert_eq!(expected, ui.read_at_cursor(expected.len() as u64).await?);

        Ok(())
    }
}
