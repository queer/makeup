#![deny(unsafe_code)]

pub mod component;
pub mod components;
pub mod render;
pub mod ui;

pub use component::Component;
pub use render::Renderer;
pub use ui::UI;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum DrawCommand {
    TextUnderCursor(String),
    TextAt { x: usize, y: usize, text: String },
}

#[cfg(test)]
mod tests {
    use crate::render::MemoryRenderer;
    use crate::{Component, DrawCommand, UI};

    use async_trait::async_trait;
    use eyre::Result;

    #[derive(Debug)]
    struct BasicComponent<'a> {
        #[allow(dead_code)]
        state: (),
        children: Vec<&'a mut dyn Component<'a, Message = ()>>,
    }

    #[async_trait]
    impl<'a> Component<'a> for BasicComponent<'a> {
        type Message = ();

        async fn render(&self) -> Result<Vec<DrawCommand>> {
            Ok(vec![DrawCommand::TextUnderCursor(
                "henol world".to_string(),
            )])
        }

        async fn on_message(&mut self, _message: Self::Message) -> Result<Option<Self::Message>> {
            Ok(Some(()))
        }

        fn children(&'a self) -> Vec<&dyn Component<'a, Message = Self::Message>> {
            self.children.iter().map(|c| &**c).collect()
        }

        fn children_mut(
            &'a mut self,
        ) -> Option<&'a mut Vec<&'a mut dyn Component<'a, Message = Self::Message>>> {
            Some(&mut self.children)
        }

        fn key(&self) -> &'a str {
            "basic"
        }
    }

    #[tokio::test]
    async fn test_it_works() -> Result<()> {
        let mut root = BasicComponent {
            state: (),
            children: vec![],
        };

        assert_eq!(
            vec![DrawCommand::TextUnderCursor("henol world".to_string(),)].as_slice(),
            root.render().await?.as_slice(),
        );

        let mut ui = UI::new(&mut root);
        let mut renderer = MemoryRenderer::new(128, 128);
        ui.render(&mut renderer).await?;

        assert_eq!("henol world", renderer.read_at_cursor(11)?);

        Ok(())
    }
}
