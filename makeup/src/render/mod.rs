use async_trait::async_trait;
use eyre::Result;
use thiserror::Error;

use crate::component::DrawCommandBatch;
use crate::util::AsAny;

pub mod memory;
pub mod terminal;

pub use memory::MemoryRenderer;

#[async_trait]
pub trait Renderer: std::fmt::Debug + AsAny {
    async fn render(&mut self, commands: &[DrawCommandBatch]) -> Result<()>;

    async fn move_cursor(&mut self, x: usize, y: usize) -> Result<()>;

    async fn move_cursor_relative(&mut self, x: isize, y: isize) -> Result<()>;

    async fn read_at_cursor(&self, width: usize) -> Result<String>;

    async fn read_string(&self, x: usize, y: usize, width: usize) -> Result<String>;

    fn cursor(&self) -> (usize, usize);
}

#[derive(Debug, Error)]
pub enum RenderError {
    #[error("Coordinates ({0}, {1}) out of bounds!")]
    OutOfBounds(isize, isize),
}

#[cfg(test)]
mod tests {
    use super::MemoryRenderer;
    use crate::components::EchoText;
    use crate::{Renderer, MUI};

    use eyre::Result;

    #[tokio::test]
    async fn test_it_works() -> Result<()> {
        let mut root = EchoText::new("henol world");

        let mut renderer = MemoryRenderer::new(128, 128);
        let ui = MUI::<()>::new(&mut root, &mut renderer);
        ui.render_frame().await?;

        renderer.move_cursor(0, 0).await?;
        assert_eq!(
            "henol world".to_string(),
            renderer.read_at_cursor(11).await?
        );

        Ok(())
    }
}
