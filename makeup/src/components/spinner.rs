use std::marker::PhantomData;
use std::time::Duration;

use async_trait::async_trait;
use either::Either;
use eyre::Result;

use crate::component::{DrawCommandBatch, Key, MakeupMessage, UpdateContext};
use crate::{Component, DrawCommand};

#[derive(Debug)]
pub struct Spinner<Message: std::fmt::Debug + Send + Sync + Clone> {
    text: String,
    spin_steps: Vec<char>,
    step: usize,
    key: Key,
    started: bool,
    _phantom: PhantomData<Message>,
}

impl<Message: std::fmt::Debug + Send + Sync + Clone> Spinner<Message> {
    pub fn new<S: Into<String>>(text: S, spin_steps: Vec<char>) -> Self {
        Self {
            text: text.into(),
            spin_steps,
            step: 0,
            key: crate::component::generate_key(),
            started: false,
            _phantom: PhantomData,
        }
    }
}

#[async_trait]
impl<Message: std::fmt::Debug + Send + Sync + Clone + 'static> Component for Spinner<Message> {
    type Message = Message;

    async fn update(&mut self, ctx: &mut UpdateContext<Self>) -> Result<()> {
        if !self.started {
            self.started = true;
            let sender = ctx.1.clone();
            let sender = sender.lock().await;
            let key = self.key();
            sender.send((key, Either::Right(MakeupMessage::TimerTick(250))))?;
        }

        if let Some(mailbox) = ctx.0.mailbox(self) {
            for msg in mailbox.iter() {
                match msg {
                    Either::Left(_msg) => {
                        // log::debug!("Spinner received message: {:?}", msg);
                    }
                    Either::Right(msg) => match msg {
                        MakeupMessage::TimerTick(_) => {
                            self.step = (self.step + 1) % self.spin_steps.len();
                            let key = self.key();
                            let sender = ctx.1.clone();
                            tokio::spawn(async move {
                                tokio::time::sleep(Duration::from_millis(250)).await;
                                let sender = sender.lock().await;
                                match sender
                                    .send((key, Either::Right(MakeupMessage::TimerTick(250))))
                                {
                                    Ok(_) => {}
                                    Err(err) => {
                                        dbg!(&err);
                                    }
                                }
                            });
                        }
                    },
                }
            }
            mailbox.clear();
        }
        Ok(())
    }

    async fn render(&self) -> Result<DrawCommandBatch> {
        Ok((
            self.key,
            vec![
                DrawCommand::TextUnderCursor(self.spin_steps[self.step].to_string()),
                DrawCommand::TextUnderCursor(" ".to_string()),
                DrawCommand::TextUnderCursor(self.text.clone()),
            ],
        ))
    }

    async fn update_pass(&mut self, ctx: &mut UpdateContext<Self>) -> Result<()> {
        self.update(ctx).await
    }

    async fn render_pass(&self) -> Result<Vec<DrawCommandBatch>> {
        Ok(vec![self.render().await?])
    }

    fn key(&self) -> Key {
        self.key
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::Spinner;
    use crate::component::MakeupMessage;
    use crate::post_office::PostOffice;
    use crate::{Component, DrawCommand};

    use eyre::Result;
    use tokio::sync::Mutex;

    #[tokio::test]
    async fn test_it_works() -> Result<()> {
        let mut root = Spinner::<()>::new("henol world", vec!['-', '\\', '|', '/']);
        let mut post_office = PostOffice::<()>::new();
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let sender = Arc::new(Mutex::new(tx));

        let (_k, render) = root.render().await?;
        assert_eq!(
            vec![
                DrawCommand::TextUnderCursor("-".into()),
                DrawCommand::TextUnderCursor(" ".into()),
                DrawCommand::TextUnderCursor("henol world".into(),)
            ]
            .as_slice(),
            render.as_slice(),
        );

        post_office.send_makeup(root.key(), MakeupMessage::TimerTick(1));
        post_office.send(root.key(), ());
        root.update_pass(&mut (&mut post_office, sender.clone()))
            .await?;

        let (_k, render) = root.render().await?;
        assert_eq!(
            vec![
                DrawCommand::TextUnderCursor("\\".into()),
                DrawCommand::TextUnderCursor(" ".into()),
                DrawCommand::TextUnderCursor("henol world".into(),)
            ]
            .as_slice(),
            render.as_slice(),
        );

        post_office.send_makeup(root.key(), MakeupMessage::TimerTick(1));
        post_office.send(root.key(), ());
        root.update_pass(&mut (&mut post_office, sender.clone()))
            .await?;

        let (_k, render) = root.render().await?;
        assert_eq!(
            vec![
                DrawCommand::TextUnderCursor("|".into()),
                DrawCommand::TextUnderCursor(" ".into()),
                DrawCommand::TextUnderCursor("henol world".into(),)
            ]
            .as_slice(),
            render.as_slice(),
        );

        post_office.send_makeup(root.key(), MakeupMessage::TimerTick(1));
        post_office.send(root.key(), ());
        root.update_pass(&mut (&mut post_office, sender.clone()))
            .await?;

        let (_k, render) = root.render().await?;
        assert_eq!(
            vec![
                DrawCommand::TextUnderCursor("/".into()),
                DrawCommand::TextUnderCursor(" ".into()),
                DrawCommand::TextUnderCursor("henol world".into(),)
            ]
            .as_slice(),
            render.as_slice(),
        );

        Ok(())
    }
}
