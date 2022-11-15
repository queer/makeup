use std::sync::Arc;
use std::time::Duration;

use async_recursion::async_recursion;
use either::Either;
use eyre::Result;
use makeup_console::Keypress;
use tokio::sync::mpsc::error::TryRecvError;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::sync::Mutex;
use tokio::time::Instant;

use crate::component::{DrawCommandBatch, Key, MakeupMessage, RenderContext, UpdateContext};
use crate::post_office::PostOffice;
use crate::util::RwLocked;
use crate::{Ansi, Component, DisplayEraseMode, Renderer};

/// A makeup UI. Generally used with [`crate::render::TerminalRenderer`].
///
/// MUIs are supposed to be entirely async. Components are updated and rendered
/// async; any blocking component tasks are expected to be moved onto the async
/// runtime's executor pool via [`tokio::spawn`] or equivalent, and then send
/// messages back to the UI via the [`PostOffice`].
#[derive(Debug)]
pub struct MUI<'a, M: std::fmt::Debug + Send + Sync + Clone + 'static> {
    ui: Arc<Mutex<UI<'a, M>>>,
    renderer: RwLocked<&'a mut dyn Renderer>,
    input_tx: Arc<Mutex<UnboundedSender<InputFrame>>>,
    input_rx: Arc<Mutex<UnboundedReceiver<InputFrame>>>,
}

impl<'a, M: std::fmt::Debug + Send + Sync + Clone> MUI<'a, M> {
    /// Create a new makeup UI with the given root component and renderer.
    pub fn new(root: &'a mut dyn Component<Message = M>, renderer: &'a mut dyn Renderer) -> Self {
        let (input_tx, input_rx) = tokio::sync::mpsc::unbounded_channel();

        Self {
            ui: Arc::new(Mutex::new(UI::new(root))),
            renderer: RwLocked::new(renderer),
            input_tx: Arc::new(Mutex::new(input_tx)),
            input_rx: Arc::new(Mutex::new(input_rx)),
        }
    }

    /// Render this MUI in a loop, forever. This will:
    /// - Move the cursor to (0, 0)
    /// - Enter alternate screen mode
    /// - Clear the screen
    /// - Update components by applying any `Mailbox`es
    /// - Render the UI
    /// The MUI will attempt to render at 60fps, sleeping as needed to stay at
    /// the frame target.
    #[allow(unreachable_code)]
    pub async fn render(&'a self, screen: bool) -> Result<()> {
        if screen {
            // Enter alternate screen
            print!("\x1b[?1049h");
            // Clear screen
            print!("{}", Ansi::EraseInDisplay(DisplayEraseMode::All));
        }

        let input_tx_lock = self.input_tx.clone();
        tokio::spawn(async move {
            loop {
                let tx = input_tx_lock.lock().await;
                match makeup_console::next_keypress().await {
                    Ok(key) => {
                        tx.send(InputFrame::Frame(key)).unwrap();
                    }
                    Err(e) => {
                        if let Some(err) = e.chain().next() {
                            match err.downcast_ref() {
                                Some(makeup_console::ConsoleError::Io(e)) => {
                                    if e.kind() == std::io::ErrorKind::UnexpectedEof {
                                        tx.send(InputFrame::End).unwrap();
                                        break;
                                    }
                                }
                                Some(makeup_console::ConsoleError::Interrupted) => {
                                    tx.send(InputFrame::End).unwrap();
                                    break;
                                }
                                _ => {}
                            }
                        }
                        eprintln!("Error: {}", e);
                    }
                }
            }
        });

        let fps_target = 60;
        let one_second_in_micros = Duration::from_secs(1).as_micros();
        let frame_target = Duration::from_micros((one_second_in_micros as u64) / fps_target);
        let mut last_frame_time = None;
        let mut last_fps: f64 = 0f64;
        let mut effective_fps: f64 = 0f64;
        let mut frame_counter = 0u128;
        'render_loop: loop {
            let start = Instant::now();
            let render_context = RenderContext {
                last_frame_time,
                frame_counter,
                fps: last_fps,
                effective_fps,
            };

            let mut pending_input = vec![];
            let mut rx = self.input_rx.lock().await;

            loop {
                match rx.try_recv() {
                    Ok(InputFrame::Frame(key)) => {
                        pending_input.push(key);
                    }
                    Ok(InputFrame::End) => {
                        break 'render_loop;
                    }
                    Err(TryRecvError::Disconnected) => {
                        eprintln!("Error: Input disconnected!?");
                        break 'render_loop;
                    }
                    Err(_) => {
                        break;
                    }
                }
            }

            if let Err(e) = self.render_frame(&pending_input, &render_context).await {
                // TODO: Handle gracefully
                eprintln!("Error: {}", e);
                break 'render_loop;
            }
            frame_counter += 1;

            let elapsed = start.elapsed();
            last_frame_time = Some(elapsed);
            effective_fps = (one_second_in_micros as f64) / (elapsed.as_micros() as f64);
            last_fps = if effective_fps as u64 > fps_target {
                fps_target as f64
            } else {
                effective_fps
            };

            // TODO: This fucks input doesn't it?
            if let Some(duration) = frame_target.checked_sub(elapsed) {
                tokio::time::sleep(duration).await
            } else {
                // log::warn!("Frame took too long to render: {:?}", elapsed);
            }
        }
        if screen {
            // Leave alternate screen
            print!("\x1b[?1049l");
        }
        Ok(())
    }

    pub async fn render_once(&'a self) -> Result<()> {
        let ctx = RenderContext {
            last_frame_time: None,
            frame_counter: 0,
            fps: 0f64,
            effective_fps: 0f64,
        };

        self.render_frame(&[], &ctx).await
    }

    /// Apply any pending `Mailbox`es and render the current frame. Makes no
    /// guarantees about hitting a framerate target, but instead renders as
    /// fast as possible.
    async fn render_frame(&'a self, pending_input: &[Keypress], ctx: &RenderContext) -> Result<()> {
        let ui = self.ui.lock().await;
        let commands = ui.render(pending_input, ctx).await?;

        let mut renderer = self.renderer.write().await;
        renderer.render(&commands).await?;

        Ok(())
    }

    /// Send a message to the given component.
    pub async fn send(&self, key: Key, message: M) {
        let ui = self.ui.lock().await;
        ui.send(key, message).await;
    }

    /// Get a mutable referenced to the renderer.
    pub fn renderer(&self) -> &RwLocked<&'a mut dyn Renderer> {
        &self.renderer
    }
}

#[derive(Debug, Clone)]
enum InputFrame {
    Frame(Keypress),
    End,
}

#[derive(Debug)]
struct UI<'a, M: std::fmt::Debug + Send + Sync + Clone> {
    root: Mutex<&'a mut dyn Component<Message = M>>,
    post_office: Arc<RwLocked<PostOffice<M>>>,
}

impl<'a, M: std::fmt::Debug + Send + Sync + Clone + 'static> UI<'a, M> {
    /// Build a new `UI` from the given root component.
    pub fn new(root: &'a mut dyn Component<Message = M>) -> Self {
        Self {
            root: Mutex::new(root),
            post_office: Arc::new(RwLocked::new(PostOffice::new())),
        }
    }

    /// Render the entire UI.
    // TODO: Graceful error handling...
    pub async fn render(
        &self,
        pending_input: &[Keypress],
        ctx: &RenderContext,
    ) -> Result<Vec<DrawCommandBatch>> {
        let mut post_office = self.post_office.write().await;
        let mut root = self.root.lock().await;
        Self::mail_pending_input(pending_input, &mut post_office, root.key());
        Self::update_recursive(*root, &mut post_office, self.post_office.clone()).await?;
        let draw_commands = root.render_pass(ctx).await?;
        Ok(draw_commands)
    }

    fn mail_pending_input(
        pending_input: &[Keypress],
        post_office: &mut PostOffice<M>,
        focused_component: Key,
    ) {
        for keypress in pending_input {
            post_office.send_makeup(focused_component, MakeupMessage::Keypress(keypress.clone()));
        }
    }

    #[async_recursion]
    async fn update_recursive(
        component: &mut dyn Component<Message = M>,
        post_office: &mut PostOffice<M>,
        post_office_lock: Arc<RwLocked<PostOffice<M>>>,
    ) -> Result<()> {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let sender = Arc::new(Mutex::new(tx));

        let mut pending_update = UpdateContext {
            post_office: &mut *post_office,
            tx: sender.clone(),
        };

        (*component).update_pass(&mut pending_update).await?;

        let lock_clone = post_office_lock.clone();
        tokio::spawn(async move {
            while let Some((id, message)) = rx.recv().await {
                let mut post_office = lock_clone.write().await;
                match message {
                    Either::Left(left) => {
                        post_office.send(id, left);
                    }
                    Either::Right(right) => {
                        post_office.send_makeup(id, right);
                    }
                }
            }
        });

        Ok(())
    }

    #[allow(unused)]
    pub async fn send(&self, key: Key, message: M) {
        let mut post_office = self.post_office.write().await;
        post_office.send(key, message);
    }

    #[allow(unused)]
    pub async fn send_makeup(&self, key: Key, message: MakeupMessage) {
        let mut post_office = self.post_office.write().await;
        post_office.send_makeup(key, message);
    }
}

#[cfg(test)]
mod tests {
    use crate::component::{
        DrawCommandBatch, ExtractMessageFromComponent, Key, RenderContext, UpdateContext,
    };
    use crate::render::MemoryRenderer;
    use crate::{Component, DrawCommand, MUI};

    use async_trait::async_trait;
    use either::Either;
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

        async fn update(
            &mut self,
            ctx: &mut UpdateContext<ExtractMessageFromComponent<Self>>,
        ) -> Result<()> {
            if let Some(mailbox) = ctx.post_office.mailbox(self) {
                for msg in mailbox.iter() {
                    if let Either::Left(cmd) = msg {
                        if cmd == "ping" {
                            self.was_pinged = true;
                        }
                    }
                }
                mailbox.clear();
            }

            Ok(())
        }

        async fn render(&self, _ctx: &RenderContext) -> Result<DrawCommandBatch> {
            if !self.was_pinged {
                Ok((
                    self.key,
                    vec![DrawCommand::TextUnderCursor("ping?".to_string())],
                ))
            } else {
                Ok((
                    self.key,
                    vec![DrawCommand::TextUnderCursor("pong!".to_string())],
                ))
            }
        }

        async fn update_pass(
            &mut self,
            ctx: &mut UpdateContext<ExtractMessageFromComponent<Self>>,
        ) -> Result<()> {
            self.update(ctx).await
        }

        async fn render_pass(&self, ctx: &RenderContext) -> Result<Vec<DrawCommandBatch>> {
            Ok(vec![self.render(ctx).await?])
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
        ui.render_once().await?;

        {
            let mut renderer = ui.renderer().write().await;
            renderer.move_cursor(0, 0).await?;
            assert_eq!("ping?".to_string(), renderer.read_at_cursor(5).await?);
        }

        ui.send(key, "ping".to_string()).await;
        ui.render_once().await?;

        {
            let mut renderer = ui.renderer().write().await;
            renderer.move_cursor(0, 0).await?;
            assert_eq!("pong!".to_string(), renderer.read_at_cursor(5).await?);
        }

        Ok(())
    }
}
