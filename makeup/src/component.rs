use async_trait::async_trait;
use eyre::Result;

use crate::DrawCommand;

/// A component in a makeup UI. Stateless components can be implemented via
/// `Self::State = ()`.
#[async_trait]
pub trait Component<'a>: std::fmt::Debug + Send + Sync {
    /// The type of messages that can be sent to this component.
    type Message: std::fmt::Debug + Send + Sync;

    /// Render this component.
    async fn render(&self) -> Result<Vec<DrawCommand>>;

    /// Send a message to this component. May choose to return a response.
    async fn on_message(&mut self, message: Self::Message) -> Result<Option<Self::Message>>;

    /// A read-only view of the component's children.
    fn children(&'a self) -> Vec<&'a dyn Component<'a, Message = Self::Message>>;

    /// A mutable view of the component's children.
    fn children_mut(
        &'a mut self,
    ) -> &'a mut Vec<&'a mut dyn Component<'a, Message = Self::Message>>;

    fn key(&self) -> &'a str;
}
