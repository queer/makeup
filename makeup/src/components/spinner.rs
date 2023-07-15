use std::marker::PhantomData;
use std::time::Duration;

use async_trait::async_trait;
use either::Either;
use eyre::Result;

use crate::component::{
    DrawCommandBatch, ExtractMessageFromComponent, Key, MakeupMessage, RenderContext, UpdateContext,
};
use crate::{check_mail, Component, DrawCommand};

/// A simple component that renders a spinner with the given text.
#[derive(Debug)]
pub struct Spinner<Message: std::fmt::Debug + Send + Sync + Clone> {
    text: String,
    spin_steps: Vec<char>,
    step: usize,
    key: Key,
    started: bool,
    interval: Duration,
    _phantom: PhantomData<Message>,
}

impl<Message: std::fmt::Debug + Send + Sync + Clone> Spinner<Message> {
    pub fn new<S: Into<String>>(text: S, spin_steps: Vec<char>, interval: Duration) -> Self {
        Self {
            text: text.into(),
            spin_steps,
            step: 0,
            key: crate::component::generate_key(),
            started: false,
            interval,
            _phantom: PhantomData,
        }
    }
}

#[async_trait]
impl<Message: std::fmt::Debug + Send + Sync + Clone + 'static> Component for Spinner<Message> {
    type Message = Message;

    fn children(&self) -> Option<Vec<&dyn Component<Message = Self::Message>>> {
        None
    }

    async fn update(
        &mut self,
        ctx: &mut UpdateContext<ExtractMessageFromComponent<Self>>,
    ) -> Result<()> {
        if !self.started {
            ctx.sender
                .send_makeup_message(self.key(), MakeupMessage::TimerTick(self.interval))?;
            self.started = true;
        }

        check_mail!(
            self,
            ctx,
            match _ {
                MakeupMessage::TimerTick(_) => {
                    self.step = (self.step + 1) % self.spin_steps.len();
                    #[cfg(not(test))]
                    ctx.sender.send_makeup_message_after(
                        self.key(),
                        MakeupMessage::TimerTick(self.interval),
                        self.interval,
                    )?;
                }
            }
        );

        Ok(())
    }

    async fn render(&self, _ctx: &RenderContext) -> Result<DrawCommandBatch> {
        self.batch(vec![
            DrawCommand::CharUnderCursor(self.spin_steps[self.step]),
            DrawCommand::CharUnderCursor(' '),
            DrawCommand::TextUnderCursor(self.text.clone()),
        ])
    }

    async fn update_pass(
        &mut self,
        ctx: &mut UpdateContext<ExtractMessageFromComponent<Self>>,
    ) -> Result<()> {
        self.update(ctx).await
    }

    async fn render_pass(&self, ctx: &RenderContext) -> Result<Vec<DrawCommandBatch>> {
        Ok(vec![self.render(ctx).await?])
    }

    fn key(&self) -> Key {
        self.key
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::Spinner;
    use crate::component::{MakeupMessage, MessageSender, UpdateContext};
    use crate::post_office::PostOffice;
    use crate::{Component, DrawCommand};

    use eyre::Result;

    #[tokio::test]
    async fn test_it_works() -> Result<()> {
        let interval = Duration::from_millis(1);
        let mut root = Spinner::<()>::new("henol world", vec!['-', '\\', '|', '/'], interval);
        let mut post_office = PostOffice::<()>::new();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();

        let (_k, render) = root.render(&crate::fake_render_ctx()).await?;
        assert_eq!(
            vec![
                DrawCommand::CharUnderCursor('-'),
                DrawCommand::CharUnderCursor(' '),
                DrawCommand::TextUnderCursor("henol world".into(),)
            ]
            .as_slice(),
            render.as_slice(),
        );

        post_office.send_makeup(root.key(), MakeupMessage::TimerTick(interval));
        post_office.send(root.key(), ());

        let mut ctx = UpdateContext {
            post_office: &mut post_office,
            sender: MessageSender::new(tx.clone(), root.key()),
            focus: root.key(),
        };
        root.update_pass(&mut ctx).await?;

        let (_k, render) = root.render(&crate::fake_render_ctx()).await?;
        assert_eq!(
            vec![
                DrawCommand::CharUnderCursor('\\'),
                DrawCommand::CharUnderCursor(' '),
                DrawCommand::TextUnderCursor("henol world".into(),)
            ]
            .as_slice(),
            render.as_slice(),
        );

        post_office.send_makeup(root.key(), MakeupMessage::TimerTick(interval));
        post_office.send(root.key(), ());

        let mut ctx = UpdateContext {
            post_office: &mut post_office,
            sender: MessageSender::new(tx.clone(), root.key()),
            focus: root.key(),
        };
        root.update_pass(&mut ctx).await?;

        let (_k, render) = root.render(&crate::fake_render_ctx()).await?;
        assert_eq!(
            vec![
                DrawCommand::CharUnderCursor('|'),
                DrawCommand::CharUnderCursor(' '),
                DrawCommand::TextUnderCursor("henol world".into(),)
            ]
            .as_slice(),
            render.as_slice(),
        );

        post_office.send_makeup(root.key(), MakeupMessage::TimerTick(interval));
        post_office.send(root.key(), ());

        let mut ctx = UpdateContext {
            post_office: &mut post_office,
            sender: MessageSender::new(tx.clone(), root.key()),
            focus: root.key(),
        };
        root.update_pass(&mut ctx).await?;

        let (_k, render) = root.render(&crate::fake_render_ctx()).await?;
        assert_eq!(
            vec![
                DrawCommand::CharUnderCursor('/'),
                DrawCommand::CharUnderCursor(' '),
                DrawCommand::TextUnderCursor("henol world".into(),)
            ]
            .as_slice(),
            render.as_slice(),
        );

        Ok(())
    }
}
