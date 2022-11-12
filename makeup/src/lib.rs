#[allow(unused)]
#[deny(unsafe_code)]
use async_trait::async_trait;

pub mod component;
pub mod ui;

pub use component::Component;
pub use ui::UI;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum DrawCommand {
    TextUnderCursor(String),
    TextAt { x: usize, y: usize, text: String },
}

#[cfg(test)]
mod tests {
    use super::{Component, DrawCommand, UI};

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
        ) -> &'a mut Vec<&'a mut dyn Component<'a, Message = Self::Message>> {
            &mut self.children
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

        let mut ui = UI::new(&mut root);

        assert_eq!(
            vec![DrawCommand::TextUnderCursor("henol world".to_string(),)].as_slice(),
            ui.render().await?.as_slice(),
        );

        Ok(())
    }
}
