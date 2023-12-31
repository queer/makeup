use std::fmt::Display;
use std::marker::PhantomData;

use async_trait::async_trait;
use eyre::Result;

use crate::component::{DrawCommandBatch, Key, MakeupMessage, MakeupUpdate, RenderContext};
use crate::{check_mail, Component, Dimensions, DrawCommand};

/// A simple component that renders text under the cursor.
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

    fn children(&self) -> Option<Vec<&Box<dyn Component<Message = Self::Message>>>> {
        None
    }

    fn children_mut(&mut self) -> Option<Vec<&mut Box<dyn Component<Message = Self::Message>>>> {
        None
    }

    async fn update(&mut self, ctx: &mut MakeupUpdate<Self>) -> Result<()> {
        check_mail!(
            self,
            ctx,
            match _ {
                MakeupMessage::TextUpdate(text) => {
                    self.text = text.clone();
                }
            }
        );

        Ok(())
    }

    async fn render(&self, _ctx: &RenderContext) -> Result<DrawCommandBatch> {
        self.batch(vec![DrawCommand::TextUnderCursor(self.text.clone())])
    }

    fn key(&self) -> Key {
        self.key
    }

    fn dimensions(&self) -> Result<Option<Dimensions>> {
        // TODO: Newlines?
        Ok(Some((self.text.len() as u64, 1)))
    }
}

impl Display for EchoText<()> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.text.fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use super::EchoText;
    use crate::test::{assert_renders_one, static_text};

    use eyre::Result;

    #[tokio::test]
    async fn test_it_works() -> Result<()> {
        let mut root = EchoText::<()>::new("henol world");
        assert_renders_one!(static_text!("henol world"), root);

        Ok(())
    }

    #[test]
    fn test_to_string() -> Result<()> {
        let root = EchoText::<()>::new("henol world");
        assert_eq!(root.to_string(), "henol world");
        assert_eq!(format!("{root}"), "henol world");
        Ok(())
    }
}
