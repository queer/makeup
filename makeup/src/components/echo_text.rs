use async_trait::async_trait;
use eyre::Result;

use crate::{Component, DrawCommand};

#[derive(Debug)]
pub struct EchoText<'a> {
    text: &'a str,
}

impl<'a> EchoText<'a> {
    pub fn new(text: &'a str) -> Self {
        Self { text }
    }
}

#[async_trait]
impl<'a> Component<'a> for EchoText<'a> {
    type Message = ();

    async fn render(&self) -> Result<Vec<DrawCommand>> {
        Ok(vec![DrawCommand::TextUnderCursor(self.text.to_string())])
    }

    async fn on_message(&mut self, _message: Self::Message) -> Result<Option<Self::Message>> {
        Ok(None)
    }

    fn children(&'a self) -> Vec<&'a dyn Component<'a, Message = Self::Message>> {
        vec![]
    }

    fn children_mut(
        &'a mut self,
    ) -> Option<&'a mut Vec<&'a mut dyn Component<'a, Message = Self::Message>>> {
        None
    }

    fn key(&self) -> &'a str {
        "echo_text"
    }
}

#[cfg(test)]
mod tests {
    use super::EchoText;
    use crate::{Component, DrawCommand};

    use eyre::Result;

    #[tokio::test]
    async fn test_it_works() -> Result<()> {
        let root = EchoText::new("henol world");

        assert_eq!(
            vec![DrawCommand::TextUnderCursor("henol world".to_string(),)].as_slice(),
            root.render().await?.as_slice(),
        );

        Ok(())
    }
}
