use std::collections::HashMap;
use std::time::Duration;

use async_recursion::async_recursion;
use async_trait::async_trait;
use derivative::Derivative;
use eyre::Result;
use taffy::prelude::*;

use crate::component::{DrawCommandBatch, Key, MakeupMessage, MakeupUpdate, RenderContext};
use crate::{check_mail, Component, Dimensions, DrawCommand};

#[derive(Derivative)]
#[derivative(Debug)]
pub struct Container<Message: std::fmt::Debug + Send + Sync + Clone> {
    children: Vec<Box<dyn Component<Message = Message>>>,
    key: Key,
    updating: bool,
    #[derivative(Debug = "ignore")]
    layout: Layout,
}

struct Layout {
    taffy: Taffy,
    taffy_leaves: HashMap<Key, Node>,
    root_node: Option<Node>,
    style: Style,
}

impl<Message: std::fmt::Debug + Send + Sync + Clone> Container<Message> {
    pub fn new(children: Vec<Box<dyn Component<Message = Message>>>, style: Option<Style>) -> Self {
        let style = style.unwrap_or_default();

        Self {
            children,
            key: crate::component::generate_key(),
            updating: false,
            layout: Layout {
                taffy: Taffy::new(),
                taffy_leaves: HashMap::new(),
                root_node: None,
                style,
            },
        }
    }

    async fn recalculate_layout(&mut self, ctx: &MakeupUpdate<'_, Self>) -> Result<()> {
        self.layout.taffy_leaves.clear();
        self.layout.taffy.clear();
        // TODO: Don't use ctx dimensions unless style dimensions are zero
        let (width, height) = ctx.dimensions;
        let root_node = self.layout.taffy.new_leaf(Style {
            size: Size {
                // TODO: Overflow???
                width: Dimension::Points(width as f32),
                height: Dimension::Points(height as f32),
            },
            ..self.layout.style.clone() // TODO: Can we be rid of this clone?
        })?;
        self.layout.root_node = Some(root_node);

        for child in &self.children {
            Self::recalculate_child_layouts(&mut self.layout, child.as_ref(), root_node).await?;
        }
        self.layout.taffy.compute_layout(
            root_node,
            Size {
                width: AvailableSpace::Definite(width as f32),
                height: AvailableSpace::Definite(height as f32),
            },
        )?;

        Ok(())
    }

    #[async_recursion]
    async fn recalculate_child_layouts(
        layout: &mut Layout,
        component: &dyn Component<Message = Message>,
        parent_node: Node,
    ) -> Result<()> {
        let child_node = Self::add_one_child(layout, component, parent_node)?;
        if let Some(children) = component.children() {
            for child in children {
                let node = Self::add_one_child(layout, component, child_node)?;
                Self::recalculate_child_layouts(layout, child, node).await?;
            }
        }

        Ok(())
    }

    fn add_one_child(
        layout: &mut Layout,
        component: &dyn Component<Message = Message>,
        parent_node: Node,
    ) -> Result<Node> {
        let dimensions = component.dimensions()?;
        let node = layout.taffy.new_leaf(Style {
            // TODO: Handle overflow?
            // TODO: What about a component like `fps` that has a really dynamic size?
            // TODO: Handle (0,0) components?
            size: Size {
                width: Dimension::Points(dimensions.0 as f32),
                height: Dimension::Points(dimensions.1 as f32),
            },
            ..Default::default()
        })?;
        layout.taffy_leaves.insert(component.key(), node);
        layout.taffy.add_child(parent_node, node)?;
        Ok(node)
    }

    #[async_recursion]
    async fn render_recursive(
        &self,
        component: &dyn Component<Message = Message>,
        ctx: &RenderContext,
    ) -> Result<Vec<DrawCommand>> {
        let (key, mut first_batch) = component.render(ctx).await?;
        let node = self
            .layout
            .taffy_leaves
            .get(&key)
            .unwrap_or_else(|| panic!("no node for child with key {key}!?"));
        let layout = self.layout.taffy.layout(*node)?;

        let mut batch = vec![DrawCommand::MoveCursorAbsolute {
            x: layout.location.x as u64,
            y: layout.location.y as u64,
        }];
        batch.append(&mut first_batch);

        for child in component.children().unwrap_or_default() {
            let mut next_batch = self.render_recursive(child, ctx).await?;
            let node = self
                .layout
                .taffy_leaves
                .get(&child.key())
                .unwrap_or_else(|| panic!("no node for child with key {key}!?"));
            let layout = self.layout.taffy.layout(*node)?;

            let mut batch = vec![DrawCommand::MoveCursorAbsolute {
                x: layout.location.x as u64,
                y: layout.location.y as u64,
            }];
            batch.append(&mut next_batch);
        }

        Ok(batch)
    }
}

#[async_trait]
impl<Message: std::fmt::Debug + Send + Sync + Clone> Component for Container<Message> {
    type Message = Message;

    fn children(&self) -> Option<Vec<&dyn Component<Message = Self::Message>>> {
        Some(self.children.iter().map(|c| c.as_ref()).collect())
    }

    async fn update(&mut self, ctx: &mut MakeupUpdate<Self>) -> Result<()> {
        if !self.updating {
            ctx.sender.send_makeup_message(
                self.key(),
                MakeupMessage::TimerTick(Duration::from_millis(100)),
            )?;
            self.updating = true;
        }

        self.recalculate_layout(ctx).await?;

        // TODO: Figure out update propagation so that containers recalculate layout when children change
        check_mail!(
            self,
            ctx,
            match _ {
                MakeupMessage::TimerTick(_) => {
                    #[cfg(not(test))]
                    ctx.sender.send_makeup_message_after(
                        self.key(),
                        MakeupMessage::TimerTick(Duration::from_millis(100)),
                        Duration::from_millis(100),
                    )?;
                }
            }
        );

        Ok(())
    }

    async fn render(&self, ctx: &RenderContext) -> Result<DrawCommandBatch> {
        let mut batches = vec![];
        for child in self.children.iter() {
            let mut next_batch = self.render_recursive(child.as_ref(), ctx).await?;
            batches.append(&mut next_batch);
        }
        Ok((self.key, batches))
    }

    async fn update_pass(&mut self, ctx: &mut MakeupUpdate<Self>) -> Result<()> {
        for child in self.children.iter_mut() {
            child.update_pass(ctx).await?;
        }
        self.update(ctx).await
    }

    async fn render_pass(&self, ctx: &RenderContext) -> Result<Vec<DrawCommandBatch>> {
        Ok(vec![self.render(ctx).await?])
    }

    fn key(&self) -> Key {
        self.key
    }

    fn dimensions(&self) -> Result<Dimensions> {
        // TODO
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::super::EchoText;
    use super::*;
    use crate::component::{MessageSender, UpdateContext};
    use crate::post_office::PostOffice;
    use crate::test::{assert_renders_one, static_text};

    use eyre::Result;

    #[tokio::test]
    async fn test_default_layout() -> Result<()> {
        let mut root = Container::<()>::new(
            vec![
                Box::new(EchoText::<()>::new("test 1")),
                Box::new(EchoText::<()>::new("test 2")),
            ],
            None,
        );

        let mut post_office = PostOffice::<()>::new();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let mut ctx = UpdateContext {
            post_office: &mut post_office,
            sender: MessageSender::new(tx.clone(), root.key()),
            focus: root.key(),
            dimensions: (100, 100),
        };
        root.update_pass(&mut ctx).await?;

        assert_renders_one!(static_text!("test 1test 2"), root);

        Ok(())
    }

    #[tokio::test]
    async fn test_vertical_layout() -> Result<()> {
        let mut root = Container::<()>::new(
            vec![
                Box::new(EchoText::<()>::new("test 1")),
                Box::new(EchoText::<()>::new("test 2")),
            ],
            Some(Style {
                flex_direction: FlexDirection::Column,
                ..Default::default()
            }),
        );

        let mut post_office = PostOffice::<()>::new();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let mut ctx = UpdateContext {
            post_office: &mut post_office,
            sender: MessageSender::new(tx.clone(), root.key()),
            focus: root.key(),
            dimensions: (100, 100),
        };
        root.update_pass(&mut ctx).await?;

        assert_renders_one!(static_text!("test 1\ntest 2"), root);

        Ok(())
    }

    #[tokio::test]
    #[should_panic]
    async fn test_vertical_layout_with_expected_horizontal() {
        async fn __do_test() -> Result<()> {
            let mut root = Container::<()>::new(
                vec![
                    Box::new(EchoText::<()>::new("test 1")),
                    Box::new(EchoText::<()>::new("test 2")),
                ],
                Some(Style {
                    flex_direction: FlexDirection::Column,
                    ..Default::default()
                }),
            );

            let mut post_office = PostOffice::<()>::new();
            let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
            let mut ctx = UpdateContext {
                post_office: &mut post_office,
                sender: MessageSender::new(tx.clone(), root.key()),
                focus: root.key(),
                dimensions: (100, 100),
            };
            root.update_pass(&mut ctx).await?;

            assert_renders_one!(static_text!("test 1test 2"), root);

            Ok(())
        }

        __do_test().await.unwrap();
    }

    #[tokio::test]
    #[should_panic]
    async fn test_horizontal_layout_with_expected_vertical() {
        async fn __do_test() -> Result<()> {
            let mut root = Container::<()>::new(
                vec![
                    Box::new(EchoText::<()>::new("test 1")),
                    Box::new(EchoText::<()>::new("test 2")),
                ],
                Some(Style {
                    flex_direction: FlexDirection::Row,
                    ..Default::default()
                }),
            );

            let mut post_office = PostOffice::<()>::new();
            let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
            let mut ctx = UpdateContext {
                post_office: &mut post_office,
                sender: MessageSender::new(tx.clone(), root.key()),
                focus: root.key(),
                dimensions: (100, 100),
            };
            root.update_pass(&mut ctx).await?;

            assert_renders_one!(static_text!("test 1\ntest 2"), root);

            Ok(())
        }

        __do_test().await.unwrap();
    }
}
