use std::time::Duration;

use async_trait::async_trait;
use either::Either;
use eyre::Result;
use makeup_console::Keypress;
use tokio::sync::mpsc::UnboundedSender;

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
/// during updates.
pub type ContextTx<M> = UnboundedSender<(Key, RawComponentMessage<M>)>;

pub type MakeupUpdate<'a, C> = UpdateContext<'a, ExtractMessageFromComponent<C>>;

/// The context for a component's update lifecycle.
#[derive(Debug)]
pub struct UpdateContext<'a, M: std::fmt::Debug + Send + Sync + Clone + 'static> {
    /// The [`PostOffice`] used for receiving messages.
    pub post_office: &'a mut PostOffice<M>,
    /// Used for sending messages.
    pub sender: MessageSender<M>,
    /// The [`Key`] of the currently-focused component.
    pub focus: Key,
    /// The dimensions of the character grid.
    pub dimensions: Dimensions,
}

impl<'a, M: std::fmt::Debug + Send + Sync + Clone + 'static> UpdateContext<'a, M> {
    pub fn new(
        post_office: &'a mut PostOffice<M>,
        sender: ContextTx<M>,
        focus: Key,
        dimensions: Dimensions,
    ) -> Self {
        Self {
            post_office,
            sender: MessageSender::new(sender, focus),
            focus,
            dimensions,
        }
    }

    pub fn sender(&self) -> MessageSender<M> {
        self.sender.clone()
    }
}

// TODO: Figure out update propagation so that containers recalculate layout when children change
/// A helper for components to use for message-sending during the update loop.
/// These functions are not on the [`UpdateContext`] itself because the
/// `sender` needs to be able to be moved across threads with a `'static`
/// lifetime, and that's achieved by repeatedly cloning the `sender`.
#[derive(Debug, Clone)]
pub struct MessageSender<M: std::fmt::Debug + Send + Sync + Clone + 'static> {
    focus: Key,
    tx: ContextTx<M>,
}

impl<M: std::fmt::Debug + Send + Sync + Clone + 'static> MessageSender<M> {
    pub fn new(tx: ContextTx<M>, focus: Key) -> Self {
        Self { tx, focus }
    }

    /// Send a message to the given component.
    pub fn send_message(&self, key: Key, msg: M) -> Result<()> {
        let sender = self.tx.clone();
        tokio::spawn(async move {
            sender.send((key, Either::Left(msg))).unwrap();
        });
        Ok(())
    }

    /// Send a [`MakeupMessage`] to the given component.
    pub fn send_makeup_message(&self, key: Key, msg: MakeupMessage) -> Result<()> {
        let sender = self.tx.clone();
        tokio::spawn(async move {
            sender.send((key, Either::Right(msg))).unwrap();
        });
        Ok(())
    }

    /// Send a message to given component after waiting for the given duration.
    pub fn send_message_after(&self, key: Key, msg: M, duration: Duration) -> Result<()> {
        let sender = self.tx.clone();
        tokio::spawn(async move {
            tokio::time::sleep(duration).await;
            sender.send((key, Either::Left(msg))).unwrap();
        });
        Ok(())
    }

    /// Send a [`MakeupMessage`] to the given component after waiting for the
    /// given duration.
    pub fn send_makeup_message_after(
        &self,
        key: Key,
        msg: MakeupMessage,
        duration: Duration,
    ) -> Result<()> {
        let sender = self.tx.clone();
        tokio::spawn(async move {
            tokio::time::sleep(duration).await;
            sender.send((key, Either::Right(msg))).unwrap();
        });
        Ok(())
    }

    /// Send a message to the currently-focused component.
    pub fn send_message_to_focused(&self, msg: M) -> Result<()> {
        self.send_message(self.focus, msg)
    }

    /// Send a [`MakeupMessage`] to the currently-focused component.
    pub fn send_makeup_message_to_focused(&self, msg: MakeupMessage) -> Result<()> {
        self.send_makeup_message(self.focus, msg)
    }

    /// Send a message to the currently-focused component after waiting for the
    /// given duration.
    pub fn send_message_to_focused_after(&self, msg: M, duration: Duration) -> Result<()> {
        self.send_message_after(self.focus, msg, duration)
    }

    /// Send a [`MakeupMessage`] to the currently-focused component after
    /// waiting for the given duration.
    pub fn send_makeup_message_to_focused_after(
        &self,
        msg: MakeupMessage,
        duration: Duration,
    ) -> Result<()> {
        self.send_makeup_message_after(self.focus, msg, duration)
    }
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
    ///
    /// **NOTE:** This *intentionally* returns a borrowed box!
    #[allow(clippy::borrowed_box)]
    fn children(&self) -> Option<Vec<&Box<dyn Component<Message = Self::Message>>>>;

    /// The children this component has, but mutable. May be empty when present.
    fn children_mut(&mut self) -> Option<Vec<&mut Box<dyn Component<Message = Self::Message>>>>;

    /// Process any messages that have been sent to this component. Messages
    /// are expected to be process asynchronously, ie. any long-running
    /// operations should be [`tokio::spawn`]ed as a task.
    async fn update(&mut self, ctx: &mut MakeupUpdate<Self>) -> Result<()>;

    /// Render this component.
    async fn render(&self, ctx: &RenderContext) -> Result<DrawCommandBatch>;

    /// A unique key for this component. See [`generate_key`].
    fn key(&self) -> Key;

    /// Batch the given render commands with this component's key.
    fn batch(&self, commands: Vec<DrawCommand>) -> Result<DrawCommandBatch> {
        Ok((self.key(), commands))
    }

    /// The dimensions of this component. Coordinates are calculated
    /// automatically by the parent component that manages layout, or are
    /// implied by render order.
    fn dimensions(&self) -> Result<Dimensions>;
}

/// Generate a most-likely-unique key for a component.
pub fn generate_key() -> Key {
    rand::random::<Key>()
}
