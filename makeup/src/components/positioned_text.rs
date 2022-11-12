use async_trait::async_trait;
use eyre::Result;

use crate::{Component, DrawCommand};

#[derive(Debug)]
pub struct PositionedText {
    text: String,
    x: usize,
    y: usize,
    key: usize,
}

impl PositionedText {
    pub fn new<S: Into<String>>(text: S, x: usize, y: usize) -> Self {
        Self {
            text: text.into(),
            x,
            y,
            key: crate::component::generate_key(),
        }
    }
}

#[async_trait]
impl<'a> Component<'a> for PositionedText {
    type Message = ();

    async fn render(&self) -> Result<Vec<DrawCommand>> {
        Ok(vec![DrawCommand::TextAt {
            text: self.text.clone(),
            x: self.x,
            y: self.y,
        }])
    }

    async fn on_message(&mut self, _message: Self::Message) -> Result<Option<Self::Message>> {
        Ok(None)
    }

    fn children(&self) -> Vec<&'a dyn Component<'a, Message = Self::Message>> {
        vec![]
    }

    fn children_mut(
        &'a mut self,
    ) -> Option<&'a mut Vec<&'a mut dyn Component<'a, Message = Self::Message>>> {
        None
    }

    fn key(&self) -> usize {
        self.key
    }
}

#[cfg(test)]
mod tests {
    use super::PositionedText;
    use crate::{Component, DrawCommand};

    use eyre::Result;

    #[tokio::test]
    async fn test_it_works() -> Result<()> {
        let root = PositionedText::new("henol world", 1, 1);

        assert_eq!(
            vec![DrawCommand::TextAt {
                text: "henol world".to_string(),
                x: 1,
                y: 1
            }]
            .as_slice(),
            root.render().await?.as_slice(),
        );

        Ok(())
    }
}
