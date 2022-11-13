use std::collections::HashMap;

use crate::component::{Key, Mailbox};
use crate::Component;

#[derive(Debug)]
pub struct PostOffice<Message: std::fmt::Debug + Send + Sync + Clone> {
    boxes: HashMap<Key, Vec<Message>>,
}

impl<Message: std::fmt::Debug + Send + Sync + Clone> PostOffice<Message> {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            boxes: HashMap::new(),
        }
    }

    pub fn send(&mut self, key: Key, message: Message) {
        self.boxes.entry(key).or_default().push(message);
    }

    pub fn mailbox<C: Component<Message = Message> + ?Sized>(
        &mut self,
        component: &C,
    ) -> Option<&mut Mailbox<C>> {
        self.boxes.get_mut(&component.key())
    }
}
