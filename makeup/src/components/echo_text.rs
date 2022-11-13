use std::marker::PhantomData;

use async_trait::async_trait;
use eyre::Result;

use crate::component::{Key, UpdateContext};
use crate::{Component, DrawCommand};

#[derive(Debug)]
pub struct EchoText<Message: std::fmt::Debug + Send + Sync + Clone> {
    text: String,
    key: Key,
    _phantom: PhantomData<Message>,
}

impl<Message: std::fmt::Debug + Send + Sync + Clone> EchoText<Message> {
    pub fn new<S: Into<String>>(text: S) -> Self {
        Self {
            text: text.into(),
            key: crate::component::generate_key(),
            _phantom: PhantomData,
        }
    }
}

#[async_trait]
impl<Message: std::fmt::Debug + Send + Sync + Clone> Component for EchoText<Message> {
    type Message = Message;

    async fn update(&mut self, _ctx: &mut UpdateContext<Self>) -> Result<()> {
        Ok(())
    }

    async fn render(&self) -> Result<Vec<DrawCommand>> {
        Ok(vec![DrawCommand::TextUnderCursor(self.text.clone())])
    }

    async fn update_pass(&mut self, _ctx: &mut UpdateContext<Self>) -> Result<()> {
        Ok(())
    }

    async fn render_pass(&self) -> Result<Vec<DrawCommand>> {
        self.render().await
    }

    fn key(&self) -> Key {
        self.key
    }
}

#[cfg(test)]
mod tests {
    use super::EchoText;
    use crate::{Component, DrawCommand};

    use eyre::Result;

    #[tokio::test]
    async fn test_it_works() -> Result<()> {
        let root = EchoText::<()>::new("henol world");

        assert_eq!(
            vec![DrawCommand::TextUnderCursor("henol world".to_string(),)].as_slice(),
            root.render().await?.as_slice(),
        );

        Ok(())
    }
}
