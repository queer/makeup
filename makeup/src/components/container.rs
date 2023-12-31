use async_trait::async_trait;
use derivative::Derivative;
use eyre::Result;
use taffy::style::Style;

use crate::component::{DrawCommandBatch, Key, MakeupUpdate, RenderContext};
use crate::{Component, Dimensions};

#[derive(Derivative)]
#[derivative(Debug)]
pub struct Container<Message: std::fmt::Debug + Send + Sync + Clone> {
    children: Vec<Box<dyn Component<Message = Message>>>,
    key: Key,
    updating: bool,
    style: Option<Style>,
}

impl<Message: std::fmt::Debug + Send + Sync + Clone> Container<Message> {
    pub fn new(children: Vec<Box<dyn Component<Message = Message>>>) -> Self {
        Self::new_with_style(children, None)
    }

    pub fn new_with_style(
        children: Vec<Box<dyn Component<Message = Message>>>,
        style: Option<Style>,
    ) -> Self {
        Self {
            children,
            key: crate::component::generate_key(),
            updating: false,
            style,
        }
    }
}

#[async_trait]
impl<Message: std::fmt::Debug + Send + Sync + Clone> Component for Container<Message> {
    type Message = Message;

    fn children(&self) -> Option<Vec<&Box<dyn Component<Message = Self::Message>>>> {
        Some(self.children.iter().collect())
    }

    fn children_mut(&mut self) -> Option<Vec<&mut Box<dyn Component<Message = Self::Message>>>> {
        Some(self.children.iter_mut().collect())
    }

    async fn update(&mut self, _ctx: &mut MakeupUpdate<Self>) -> Result<()> {
        Ok(())
    }

    async fn render(&self, _ctx: &RenderContext) -> Result<DrawCommandBatch> {
        self.batch(vec![])
    }

    fn key(&self) -> Key {
        self.key
    }

    fn dimensions(&self) -> Result<Option<Dimensions>> {
        Ok(None)
    }

    fn style(&self) -> Option<taffy::style::Style> {
        self.style.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::super::EchoText;
    use super::*;
    use crate::test::{assert_renders_one, static_text};

    use eyre::Result;
    use taffy::style::FlexDirection;

    #[tokio::test]
    async fn test_default_layout() -> Result<()> {
        let mut root = Container::<()>::new(vec![
            Box::new(EchoText::<()>::new("test 1")),
            Box::new(EchoText::<()>::new("test 2")),
        ]);

        assert_renders_one!(static_text!("test 1test 2"), root);

        Ok(())
    }

    #[tokio::test]
    async fn test_vertical_layout() -> Result<()> {
        let mut root = Container::<()>::new_with_style(
            vec![
                Box::new(EchoText::<()>::new("test 1")),
                Box::new(EchoText::<()>::new("test 2")),
            ],
            Some(Style {
                flex_direction: FlexDirection::Column,
                ..Default::default()
            }),
        );

        assert_renders_one!(static_text!("test 1\ntest 2"), root);

        Ok(())
    }

    #[tokio::test]
    #[should_panic]
    async fn test_vertical_layout_with_expected_horizontal() {
        async fn __do_test() -> Result<()> {
            let mut root = Container::<()>::new_with_style(
                vec![
                    Box::new(EchoText::<()>::new("test 1")),
                    Box::new(EchoText::<()>::new("test 2")),
                ],
                Some(Style {
                    flex_direction: FlexDirection::Column,
                    ..Default::default()
                }),
            );

            assert_renders_one!(static_text!("test 1test 2"), root);

            Ok(())
        }

        __do_test().await.unwrap();
    }

    #[tokio::test]
    #[should_panic]
    async fn test_horizontal_layout_with_expected_vertical() {
        async fn __do_test() -> Result<()> {
            let mut root = Container::<()>::new_with_style(
                vec![
                    Box::new(EchoText::<()>::new("test 1")),
                    Box::new(EchoText::<()>::new("test 2")),
                ],
                Some(Style {
                    flex_direction: FlexDirection::Row,
                    ..Default::default()
                }),
            );

            assert_renders_one!(static_text!("test 1\ntest 2"), root);

            Ok(())
        }

        __do_test().await.unwrap();
    }
}
