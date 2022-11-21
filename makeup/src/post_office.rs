use std::collections::HashMap;

use either::Either;

use crate::component::{Key, Mailbox, MakeupMessage, RawComponentMessage};
use crate::ui::UiControlMessage;
use crate::Component;

/// The post office is used for managing component mailboxes, including sending
/// and receiving messages.
#[derive(Debug)]
pub struct PostOffice<Message: std::fmt::Debug + Send + Sync + Clone> {
    boxes: HashMap<Key, Vec<RawComponentMessage<Message>>>,
    ui_mailbox: Vec<UiControlMessage>,
}

impl<Message: std::fmt::Debug + Send + Sync + Clone> PostOffice<Message> {
    /// Create a new post office instance.
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            boxes: HashMap::new(),
            ui_mailbox: vec![],
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

    /// Send a UI control message to the UI message queue.
    pub fn send_control(&mut self, message: UiControlMessage) {
        self.ui_mailbox.push(message);
    }

    /// Get the mailbox for the given component.
    pub fn mailbox<C: Component<Message = Message> + ?Sized>(
        &mut self,
        component: &C,
    ) -> Option<&mut Mailbox<C>> {
        self.boxes.get_mut(&component.key())
    }

    /// Get the UI message queue.
    pub(crate) fn ui_mailbox(&self) -> &Vec<UiControlMessage> {
        &self.ui_mailbox
    }

    /// Clear the UI message queue.
    pub(crate) fn clear_ui_mailbox(&mut self) {
        self.ui_mailbox.clear();
    }
}

/// Check the mail for the current component. Clears mailboxes after reading.
#[macro_export]
macro_rules! check_mail {
    ( $component:expr, $ctx:expr, $external:pat_param => $external_handler:block ) => {
        if let Some(mailbox) = $ctx.post_office.mailbox($component) {
            for message in mailbox.iter() {
                match message {
                    Either::Left($external) => $external_handler,
                    Either::Right(_) => {}
                }
            }
        }
    };

    ( $component:expr, $ctx:expr, { $external:pat_param => $external_handler:block, $internal:pat_param => $internal_handler:block } ) => {
        if let Some(mailbox) = $ctx.post_office.mailbox($component) {
            for message in mailbox.iter() {
                match message {
                    Either::Left($external) => $external_handler,
                    Either::Right($internal) => $internal_handler,
                }
            }
            mailbox.clear();
        }
    };
}
