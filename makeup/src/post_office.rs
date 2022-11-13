use std::collections::HashMap;

use either::Either;

use crate::component::{Key, Mailbox, MakeupMessage, RawComponentMessage};
use crate::Component;

/// The post office is used for managing component mailboxes, including sending
/// and receiving messages.
#[derive(Debug)]
pub struct PostOffice<Message: std::fmt::Debug + Send + Sync + Clone> {
    boxes: HashMap<Key, Vec<RawComponentMessage<Message>>>,
}

impl<Message: std::fmt::Debug + Send + Sync + Clone> PostOffice<Message> {
    /// Create a new post office instance.
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            boxes: HashMap::new(),
        }
    }

    /// Send a message to the mailbox with the given key.
    pub fn send(&mut self, key: Key, message: Message) {
        self.boxes
            .entry(key)
            .or_default()
            .push(Either::Left(message));
    }

    /// Send an internal (makeup) message to the mailbox with the given key.
    pub fn send_makeup(&mut self, key: Key, message: MakeupMessage) {
        self.boxes
            .entry(key)
            .or_default()
            .push(Either::Right(message));
    }

    /// Get the mailbox for the given component.
    pub fn mailbox<C: Component<Message = Message> + ?Sized>(
        &mut self,
        component: &C,
    ) -> Option<&mut Mailbox<C>> {
        self.boxes.get_mut(&component.key())
    }
}
