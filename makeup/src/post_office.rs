use std::collections::HashMap;

use either::Either;

use crate::component::{Key, Mailbox, MakeupMessage, RawComponentMessage};
use crate::Component;

#[derive(Debug)]
pub struct PostOffice<Message: std::fmt::Debug + Send + Sync + Clone> {
    boxes: HashMap<Key, Vec<RawComponentMessage<Message>>>,
}

impl<Message: std::fmt::Debug + Send + Sync + Clone> PostOffice<Message> {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            boxes: HashMap::new(),
        }
    }

    pub fn send(&mut self, key: Key, message: Message) {
        self.boxes
            .entry(key)
            .or_default()
            .push(Either::Left(message));
    }

    pub fn send_makeup(&mut self, key: Key, message: MakeupMessage) {
        self.boxes
            .entry(key)
            .or_default()
            .push(Either::Right(message));
    }

    pub fn mailbox<C: Component<Message = Message> + ?Sized>(
        &mut self,
        component: &C,
    ) -> Option<&mut Mailbox<C>> {
        self.boxes.get_mut(&component.key())
    }
}
