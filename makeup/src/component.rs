use std::sync::Arc;

use async_trait::async_trait;
use either::Either;
use eyre::Result;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::Mutex;

use crate::{post_office::PostOffice, DrawCommand};

pub type Key = usize;
pub type DrawCommandBatch = (Key, Vec<DrawCommand>);

pub type RawComponentMessage<M> = Either<M, MakeupMessage>;
pub type ExtractMessageFromComponent<C> = <C as Component>::Message;
pub type ComponentMessage<C> = RawComponentMessage<ExtractMessageFromComponent<C>>;
pub type Mailbox<C> = Vec<ComponentMessage<C>>;
pub type ContextSender<C> = Arc<Mutex<UnboundedSender<(Key, ComponentMessage<C>)>>>;
// TODO: Figure out the type magic to lift this into a real struct
pub type UpdateContext<'a, C> = (
    &'a mut PostOffice<ExtractMessageFromComponent<C>>,
    ContextSender<C>,
);

#[derive(Debug, Clone)]
pub enum MakeupMessage {
    TimerTick(usize),
}

/// A component in a makeup UI. Stateless components can be implemented via
/// `Self::State = ()`.
#[async_trait]
pub trait Component: std::fmt::Debug + Send + Sync {
    /// The type of messages that can be sent to this component.
    type Message: std::fmt::Debug + Send + Sync + Clone;

    /// Process any messages that have been sent to this component. Messages
    /// are expected to be process asynchronously, ie. any long-running
    /// operations should be [`tokio::spawn`]ed as a task.
    async fn update(&mut self, mailbox: &mut UpdateContext<Self>) -> Result<()>;

    /// Render this component.
    async fn render(&self) -> Result<DrawCommandBatch>;

    /// An update pass for this component. Generally, this is implemented by
    /// calling [`Self::update`] and calling `::update` on any child
    /// components.
    async fn update_pass(&mut self, ctx: &mut UpdateContext<Self>) -> Result<()>;

    /// A render pass for this component. Generally, this is implemented by
    /// invoking `self.render()` and then calling `render` on each child.
    async fn render_pass(&self) -> Result<Vec<DrawCommandBatch>>;

    /// A unique key for this component. See [`generate_key`].
    fn key(&self) -> Key;
}

pub fn generate_key() -> Key {
    rand::random::<usize>()
}
