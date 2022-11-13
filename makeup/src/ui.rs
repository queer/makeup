use std::sync::Arc;

use async_recursion::async_recursion;
use eyre::Result;
use tokio::sync::Mutex;

use crate::component::Key;
use crate::post_office::PostOffice;
use crate::util::RwLocked;
use crate::{Component, DrawCommand, Renderer};

#[derive(Debug)]
pub struct MUI<'a, M: std::fmt::Debug + Send + Sync + Clone> {
    ui: Mutex<UI<'a, M>>,
    renderer: RwLocked<&'a mut dyn Renderer>,
}

impl<'a, M: std::fmt::Debug + Send + Sync + Clone> MUI<'a, M> {
    pub fn new(root: &'a mut dyn Component<Message = M>, renderer: &'a mut dyn Renderer) -> Self {
        Self {
            ui: Mutex::new(UI::new(root)),
            renderer: RwLocked::new(renderer),
        }
    }

    pub async fn render(&'a self) -> Result<()> {
        let ui = self.ui.lock().await;
        let commands = ui.render().await?;

        let mut renderer = self.renderer.write().await;
        renderer.render(&commands).await?;
        Ok(())
    }

    pub async fn send(&self, key: Key, message: M) {
        let ui = self.ui.lock().await;
        ui.send(key, message).await;
    }

    pub fn renderer(&self) -> &RwLocked<&'a mut dyn Renderer> {
        &self.renderer
    }
}

#[derive(Debug)]
struct UI<'a, M: std::fmt::Debug + Send + Sync + Clone> {
    root: RwLocked<&'a mut dyn Component<Message = M>>,
    post_office: Arc<Mutex<PostOffice<M>>>,
}

impl<'a, M: std::fmt::Debug + Send + Sync + Clone + 'a> UI<'a, M> {
    /// Build a new `UI` from the given root component.
    pub fn new(root: &'a mut dyn Component<Message = M>) -> Self {
        Self {
            root: RwLocked::new(root),
            post_office: Arc::new(Mutex::new(PostOffice::new())),
        }
    }

    /// Render the entire UI.
    // TODO: Graceful error handling...
    pub async fn render(&self) -> Result<Vec<DrawCommand>> {
        Self::update_recursive(&self.root, self.post_office.clone()).await?;
        let root = &self.root.read().await;
        let draw_commands = root.render_pass().await?;
        Ok(draw_commands)
    }

    #[async_recursion]
    async fn update_recursive(
        component: &RwLocked<&mut dyn Component<Message = M>>,
        post_office: Arc<Mutex<PostOffice<M>>>,
    ) -> Result<()> {
        let post_office = post_office.clone();
        let mut post_office = post_office.lock().await;
        let mut component = component.write().await;
        if let Some(messages) = post_office.mailbox(*component) {
            (*component).update_pass(messages).await?;
        }
        Ok(())
    }

    pub async fn send(&self, key: usize, message: M) {
        let mut post_office = self.post_office.lock().await;
        post_office.send(key, message);
    }
}

#[cfg(test)]
mod tests {
    use crate::component::{Key, Mailbox};
    use crate::render::MemoryRenderer;
    use crate::{Component, DrawCommand, MUI};

    use async_trait::async_trait;
    use eyre::Result;

    #[derive(Debug)]
    struct PingableComponent {
        #[allow(dead_code)]
        state: (),
        key: Key,
        was_pinged: bool,
    }

    #[async_trait]
    impl Component for PingableComponent {
        type Message = String;

        async fn update(&mut self, mailbox: &Mailbox<Self>) -> Result<()> {
            if let Some("ping") = mailbox.iter().next().map(|s| s.as_str()) {
                self.was_pinged = true;
            }

            Ok(())
        }

        async fn render(&self) -> Result<Vec<DrawCommand>> {
            if !self.was_pinged {
                Ok(vec![DrawCommand::TextUnderCursor("ping?".to_string())])
            } else {
                Ok(vec![DrawCommand::TextUnderCursor("pong!".to_string())])
            }
        }

        async fn update_pass(&mut self, mailbox: &Mailbox<Self>) -> Result<()> {
            self.update(mailbox).await
        }

        async fn render_pass(&self) -> Result<Vec<DrawCommand>> {
            self.render().await
        }

        fn key(&self) -> Key {
            self.key
        }
    }

    #[tokio::test]
    async fn test_messaging_works() -> Result<()> {
        let mut root = PingableComponent {
            state: (),
            key: crate::component::generate_key(),
            was_pinged: false,
        };
        let key = root.key();

        let mut renderer = MemoryRenderer::new(128, 128);
        let ui = MUI::new(&mut root, &mut renderer);
        ui.render().await?;

        {
            let mut renderer = ui.renderer().write().await;
            renderer.move_cursor(0, 0).await?;
            assert_eq!("ping?".to_string(), renderer.read_at_cursor(5).await?);
        }

        ui.send(key, "ping".to_string()).await;
        ui.render().await?;

        {
            let mut renderer = ui.renderer().write().await;
            renderer.move_cursor(0, 0).await?;
            assert_eq!("pong!".to_string(), renderer.read_at_cursor(5).await?);
        }

        Ok(())
    }
}
