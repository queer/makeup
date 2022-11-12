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
    use crate::components::EchoText;
    use crate::render::MemoryRenderer;
    use crate::{Component, DrawCommand, Renderer, UI};

    use async_trait::async_trait;
    use eyre::Result;

    #[derive(Debug)]
    struct BasicComponent<'a> {
        #[allow(dead_code)]
        state: (),
        children: Vec<&'a mut dyn Component<'a, Message = ()>>,
        key: usize,
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

        fn children(&self) -> Vec<&dyn Component<'a, Message = Self::Message>> {
            self.children.iter().map(|c| &**c).collect()
        }

        fn children_mut(
            &'a mut self,
        ) -> Option<&mut Vec<&mut dyn Component<'a, Message = Self::Message>>> {
            Some(&mut self.children)
        }

        fn key(&self) -> usize {
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

        let mut ui = UI::new(&mut root);
        let commands = ui.render().await?;
        let expected = "henol world".to_string();
        assert_eq!(
            vec![DrawCommand::TextUnderCursor(expected.clone())].as_slice(),
            commands.as_slice(),
        );

        let mut renderer = MemoryRenderer::new(128, 128);
        renderer.render(commands).await?;
        renderer.move_cursor(0, 0)?;
        assert_eq!(expected, renderer.read_at_cursor(expected.len())?);

        Ok(())
    }

    #[tokio::test]
    async fn test_it_renders_children() -> Result<()> {
        let mut child = EchoText::new("? wrong! banana!");

        let mut root = BasicComponent {
            state: (),
            children: vec![&mut child],
            key: crate::component::generate_key(),
        };

        let mut ui = UI::new(&mut root);
        let commands = ui.render().await?;
        assert_eq!(
            vec![
                DrawCommand::TextUnderCursor("henol world".to_string()),
                DrawCommand::TextUnderCursor("? wrong! banana!".to_string())
            ]
            .as_slice(),
            commands.as_slice(),
        );

        let mut renderer = MemoryRenderer::new(128, 128);
        renderer.render(commands).await?;

        let expected = "henol world? wrong! banana".to_string();
        renderer.move_cursor(0, 0)?;
        assert_eq!(expected, renderer.read_at_cursor(expected.len())?);

        Ok(())
    }
}
