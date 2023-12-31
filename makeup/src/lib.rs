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

    /// Move the cursor absolutely. This is almost certainly ***NOT*** what you
    /// want.
    MoveCursorAbsolute { x: Coordinate, y: Coordinate },

    /// Move the cursor relative to its current position. You probably want
    /// this and not `MoveCursorAbsolute`.
    MoveCursorRelative {
        x: RelativeCoordinate,
        y: RelativeCoordinate,
    },

    /// Hide the cursor.
    HideCursor,

    /// Show the cursor.
    ShowCursor,

    /// Style the text that follows this command.
    Style(DrawStyle),
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord, strum::Display)]
pub enum DrawStyle {
    /// Draw the text with the given foreground and background colours.
    Coloured {
        foreground: Option<u32>,
        background: Option<u32>,
    },

    /// Draw the text with the given foreground colour, and the default
    /// background colour.
    Foreground(u32),

    /// Draw the text with the given background colour, and the default
    /// foreground colour.
    Background(u32),

    /// Draw the text with the given foreground and background colours, using
    /// the 8-bit colour palette.
    Coloured8Bit {
        foreground: Option<Colour>,
        background: Option<Colour>,
    },

    /// Draw the text with the given foreground colour, and the default
    /// background colour, using the 8-bit colour palette.
    Foreground8Bit(Colour),

    /// Draw the text with the given background colour, and the default
    /// foreground colour, using the 8-bit colour palette.
    Background8Bit(Colour),

    /// Draw the text with the default foreground and background colours.
    /// This will undo any previous text settings.
    Default,

    /// Draw the text in bold font.
    Bold,

    /// Draw the text in italic font.
    Italic,

    /// Draw the text with an underline.
    Underline,
}

#[cfg(test)]
mod tests {
    use crate::component::{DrawCommandBatch, Key, MakeupUpdate, RenderContext};
    use crate::input::TerminalInput;
    use crate::render::MemoryRenderer;
    use crate::{Component, Dimensions, DrawCommand, MUI};

    use async_trait::async_trait;
    use eyre::Result;

    #[derive(Debug)]
    struct BasicComponent {
        #[allow(dead_code)]
        state: (),
        children: Vec<Box<dyn Component<Message = ()>>>,
        key: Key,
    }

    #[async_trait]
    impl Component for BasicComponent {
        type Message = ();

        fn children(&self) -> Option<Vec<&Box<dyn Component<Message = Self::Message>>>> {
            Some(self.children.iter().collect())
        }

        fn children_mut(
            &mut self,
        ) -> Option<Vec<&mut Box<dyn Component<Message = Self::Message>>>> {
            Some(self.children.iter_mut().collect())
        }

        async fn update(&mut self, _ctx: &mut MakeupUpdate<Self>) -> Result<()> {
            Ok(())
        }

        async fn render(&self, _ctx: &RenderContext) -> Result<DrawCommandBatch> {
            Ok((
                self.key,
                vec![DrawCommand::TextUnderCursor("henol world".into())],
            ))
        }

        fn key(&self) -> Key {
            self.key
        }

        fn dimensions(&self) -> Result<Option<Dimensions>> {
            Ok(Some((11, 1)))
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
        let ui = MUI::new(&mut root, Box::new(renderer), input)?;
        ui.render_once().await?;
        let expected = "henol world".to_string();
        ui.render_once().await?;
        ui.move_cursor(0, 0).await?;
        assert_eq!(expected, ui.read_at_cursor(expected.len() as u64).await?);

        Ok(())
    }
}
