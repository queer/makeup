use async_trait::async_trait;
use eyre::Result;
use thiserror::Error;

use crate::component::DrawCommandBatch;
use crate::util::AsAny;
use crate::{Coordinate, Coordinates, Dimension, Dimensions, RelativeCoordinate};

pub mod memory;
pub mod terminal;

pub use memory::MemoryRenderer;
pub use terminal::TerminalRenderer;

/// A `Renderer` takes in a slice of [`DrawCommandBatch`]es and renders them
/// somehow. No constraints are placed on rendering, ie a renderer can use any
/// backend it sees fit. Built-in renderers include [`MemoryRenderer`] and
/// [`TerminalRenderer`].
///
/// Renderers that might be useful to implement on your own are things like:
/// - A renderer that can render to a canvas backend, for trivial WASM parity
#[async_trait]
pub trait Renderer: std::fmt::Debug + AsAny + Send + Sync {
    async fn render(&mut self, commands: &[DrawCommandBatch]) -> Result<()>;

    async fn flush(&mut self) -> Result<()>;

    async fn move_cursor(&mut self, x: Coordinate, y: Coordinate) -> Result<()>;

    async fn move_cursor_relative(
        &mut self,
        x: RelativeCoordinate,
        y: RelativeCoordinate,
    ) -> Result<()>;

    async fn read_at_cursor(&self, width: Dimension) -> Result<String>;

    async fn read_string(&self, x: Coordinate, y: Coordinate, width: Dimension) -> Result<String>;

    fn cursor(&self) -> Coordinates;

    fn dimensions(&self) -> Dimensions;

    fn set_width(&mut self, w: Dimension);

    fn set_height(&mut self, h: Dimension);
}

/// An error that occurred during rendering.
#[derive(Debug, Error)]
pub enum RenderError {
    #[error("Coordinates ({0}, {1}) out of bounds!")]
    OutOfBounds(RelativeCoordinate, RelativeCoordinate),
}

#[cfg(test)]
mod tests {
    use super::MemoryRenderer;
    use crate::components::EchoText;
    use crate::input::TerminalInput;
    use crate::MUI;

    use eyre::Result;

    #[tokio::test]
    async fn test_it_works() -> Result<()> {
        let root = EchoText::<()>::new("henol world");

        let renderer = MemoryRenderer::new(128, 128);
        let input = TerminalInput::new().await?;
        let ui = MUI::new(Box::new(root), Box::new(renderer), input);
        ui.render_once().await?;

        ui.move_cursor(0, 0).await?;
        assert_eq!("henol world".to_string(), ui.read_at_cursor(11).await?);

        Ok(())
    }
}
