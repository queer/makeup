use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use either::Either;
use eyre::Result;
use makeup_console::Keypress;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::Mutex;

use crate::post_office::PostOffice;
use crate::{Coordinates, Dimensions, DrawCommand};

/// A key that uniquely identifies a [`Component`].
pub type Key = u64;

/// A [`Key`]ed batch of [`DrawCommand`]s.
pub type DrawCommandBatch = (Key, Vec<DrawCommand>);

/// The exact message type that can be sent to a component. Either the
/// component's associated `Message` type, or a [`MakeupMessage`].
pub type RawComponentMessage<M> = Either<M, MakeupMessage>;

/// The associated `Message` type of a [`Component`].
pub type ExtractMessageFromComponent<C> = <C as Component>::Message;

/// The type of messages that can be sent to the given [`Component`].
pub type ComponentMessage<C> = RawComponentMessage<ExtractMessageFromComponent<C>>;

/// A mailbox for a component.
pub type Mailbox<C> = Vec<ComponentMessage<C>>;

/// An [`UnboundedSender`] that can be used to send messages to a component
/// during the [`Component::update_pass`] loop.
pub type ContextTx<M> = Arc<Mutex<UnboundedSender<(Key, M)>>>;

/// The context for a component's update lifecycle.
#[derive(Debug)]
pub struct UpdateContext<'a, M: std::fmt::Debug + Send + Sync + Clone + 'a> {
    /// The [`PostOffice`] used for receiving messages.
    pub post_office: &'a mut PostOffice<M>,
    /// Used for sending messages.
    pub tx: ContextTx<RawComponentMessage<M>>,
    /// The [`Key`] of the currently-focused component.
    pub focus: Key,
}

#[derive(Debug, Clone)]
pub struct RenderContext {
    /// How long the previous frame took to render. May not be present.
    pub last_frame_time: Option<Duration>,
    /// The number of the current frame. Will only ever increase.
    pub frame_counter: u128,
    /// The last FPS value.
    pub fps: f64,
    /// The last effective FPS value. Maybe be larger than `fps`, sometimes
    /// significantly so.
    pub effective_fps: f64,
    /// The coordinates of the cursor in the character grid.
    pub cursor: Coordinates,
    /// The dimensions of the character grid.
    pub dimensions: Dimensions,
    /// The [`Key`] of the currently-focused component.
    pub focus: Key,
}

/// A default message that can be sent to a component. Contains a lot of the
/// built-in functionality you would expect:
/// - Timer ticks
/// - Text updates
#[derive(Debug, Clone)]
pub enum MakeupMessage {
    TimerTick(Duration),
    TextUpdate(String),
    Keypress(Keypress),
}

/// A component in a makeup UI. Stateless components can be implemented via
/// `Self::State = ()`.
#[async_trait]
pub trait Component: std::fmt::Debug + Send + Sync {
    /// The type of messages that can be sent to this component.
    type Message: std::fmt::Debug + Send + Sync + Clone;

    /// The children this component has. May be empty when present.
    fn children(&self) -> Option<Vec<&dyn Component<Message = Self::Message>>>;

    /// Process any messages that have been sent to this component. Messages
    /// are expected to be process asynchronously, ie. any long-running
    /// operations should be [`tokio::spawn`]ed as a task.
    async fn update(
        &mut self,
        ctx: &mut UpdateContext<ExtractMessageFromComponent<Self>>,
    ) -> Result<()>;

    /// Render this component.
    async fn render(&self, ctx: &RenderContext) -> Result<DrawCommandBatch>;

    /// An update pass for this component. Generally, this is implemented by
    /// calling [`Self::update`] and calling `::update` on any child
    /// components.
    async fn update_pass(
        &mut self,
        ctx: &mut UpdateContext<ExtractMessageFromComponent<Self>>,
    ) -> Result<()>;

    /// A render pass for this component. Generally, this is implemented by
    /// invoking `self.render()` and then calling `render` on each child.
    async fn render_pass(&self, ctx: &RenderContext) -> Result<Vec<DrawCommandBatch>>;

    /// A unique key for this component. See [`generate_key`].
    fn key(&self) -> Key;
}

/// Generate a most-likely-unique key for a component.
pub fn generate_key() -> Key {
    rand::random::<Key>()
}
