use async_trait::async_trait;
use eyre::Result;

use crate::DrawCommand;

pub type Key = usize;
pub type Mailbox<C> = Vec<<C as Component>::Message>;

/// A component in a makeup UI. Stateless components can be implemented via
/// `Self::State = ()`.
#[async_trait]
pub trait Component: std::fmt::Debug + Send + Sync {
    /// The type of messages that can be sent to this component.
    type Message: std::fmt::Debug + Send + Sync + Clone;

    /// Process any messages that have been sent to this component. Messages
    /// are expected to be process asynchronously, ie. any long-running
    /// operations should be [`tokio::spawn`]ed as a task.
    async fn update(&mut self, mailbox: &Mailbox<Self>) -> Result<()>;

    /// Render this component.
    async fn render(&self) -> Result<Vec<DrawCommand>>;

    /// An update pass for this component. Generally, this is implemented by
    /// calling [`Self::update`] and calling `::update` on any child
    /// components.
    async fn update_pass(&mut self, mailbox: &Mailbox<Self>) -> Result<()>;

    /// A render pass for this component. Generally, this is implemented by
    /// invoking `self.render()` and then calling `render` on each child.
    async fn render_pass(&self) -> Result<Vec<DrawCommand>>;

    /// A unique key for this component. See [`generate_key`].
    fn key(&self) -> Key;
}

pub fn generate_key() -> Key {
    rand::random::<usize>()
}
