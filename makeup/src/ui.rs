use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use async_recursion::async_recursion;
use derivative::Derivative;
use either::Either;
use eyre::Result;
use makeup_console::Keypress;
use taffy::prelude::*;
use tokio::sync::mpsc::error::TryRecvError;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::sync::{Mutex, RwLock};
use tokio::time::Instant;

use crate::component::{
    DrawCommandBatch, Key, MakeupMessage, MessageSender, RenderContext, UpdateContext,
};
use crate::input::{InputFrame, TerminalInput};
use crate::post_office::PostOffice;
use crate::{Ansi, Component, Coordinates, Dimensions, DisplayEraseMode, Input, Renderer};

#[derive(Debug, Clone)]
pub enum UiControlMessage {
    MoveFocus(Key),
    StopRendering,
}

#[derive(Debug)]
pub enum RenderState {
    Running,
    Stopped,
}

pub type RwLocked<T> = Arc<RwLock<T>>;

/// A makeup UI. Generally used with [`crate::render::TerminalRenderer`].
///
/// MUIs are supposed to be entirely async. Components are updated and rendered
/// async; any blocking component tasks are expected to be moved onto the async
/// runtime's executor pool via [`tokio::spawn`] or equivalent, and then send
/// messages back to the UI via the [`PostOffice`].
#[derive(Debug)]
pub struct MUI<
    'a,
    M: std::fmt::Debug + Send + Sync + Clone + 'static,
    I: Input + 'static = TerminalInput,
> {
    ui: Arc<Mutex<UI<'a, M>>>,
    renderer: RwLocked<Box<dyn Renderer>>,
    input_tx: UnboundedSender<InputFrame>,
    input_rx: Arc<Mutex<UnboundedReceiver<InputFrame>>>,
    input: I,
    done: Arc<Mutex<bool>>,
}

impl<'a, M: std::fmt::Debug + Send + Sync + Clone, I: Input + 'static> MUI<'a, M, I> {
    /// Create a new makeup UI with the given root component and renderer.
    pub fn new(
        root: Box<dyn Component<Message = M>>,
        renderer: Box<dyn Renderer>,
        input: I,
    ) -> Result<Self> {
        let (input_tx, input_rx) = tokio::sync::mpsc::unbounded_channel();

        Ok(Self {
            ui: Arc::new(Mutex::new(UI::new(root)?)),
            renderer: Arc::new(RwLock::new(renderer)),
            input_tx,
            input_rx: Arc::new(Mutex::new(input_rx)),
            input,
            done: Arc::new(Mutex::new(false)),
        })
    }

    /// Render this MUI in a loop, forever. This will:
    /// - Move the cursor to (0, 0)
    /// - Enter alternate screen mode
    /// - Clear the screen
    /// - Update components by applying any `Mailbox`es
    /// - Render the UI
    /// The MUI will attempt to render at 60fps, sleeping as needed to stay at
    /// the frame target.
    pub async fn render(&'a self, screen: bool) -> Result<RenderState> {
        {
            let done = self.done.lock().await;
            if *done {
                return Ok(RenderState::Stopped);
            }
        }
        if screen {
            // Enter alternate screen
            print!("\x1b[?1049h");
            // Clear screen
            print!("{}", Ansi::EraseInDisplay(DisplayEraseMode::All));
        }

        let mut last_frame_time = None;
        let mut last_fps: f64 = 0f64;
        let mut effective_fps: f64 = 0f64;
        let mut frame_counter = 0u128;
        let (cursor, dimensions) = {
            let renderer = self.renderer.read().await;

            (renderer.cursor(), renderer.dimensions())
        };

        // Input setup.
        // Don't want the clones escaping this scope.
        let done_for_input = self.done.clone();
        let input_handle = {
            let input = self.input.clone();
            let input_tx = self.input_tx.clone();
            tokio::spawn(async move {
                loop {
                    let frame = input.next_frame().await.unwrap();
                    let mut done = false;
                    if frame == InputFrame::End {
                        done = true;
                    }
                    if let Err(_e) = input_tx.send(frame) {
                        break;
                    }
                    if done {
                        break;
                    }
                    {
                        let done = done_for_input.lock().await;
                        if *done {
                            break;
                        }
                    }
                }
            })
        };

        'run_loop: loop {
            tokio::select! {
                update_res = self.update_loop() => {
                    if update_res.is_err() {
                        {
                            let mut done = self.done.lock().await;
                            *done = true;
                        }
                    }
                }
                render_res = self.render_loop(
                    &mut last_frame_time,
                    &mut frame_counter,
                    &mut last_fps,
                    &mut effective_fps,
                    &cursor,
                    &dimensions,
                ) => {
                    let currently_exiting = match render_res {
                        Ok(exiting) => exiting,
                        Err(_e) => true,
                    };
                    if currently_exiting {
                        {
                            let mut done = self.done.lock().await;
                            *done = true;
                        }
                    }
                }
            }

            let done = *self.done.lock().await;
            if done {
                // We have to render one last time to ensure that the cursor
                // ends up in the expected position.
                self.render_frame(&mut RenderContext {
                    last_frame_time,
                    frame_counter,
                    fps: last_fps,
                    effective_fps,
                    cursor,
                    dimensions,
                    // Default values, these are filled in by the inner render method.
                    focus: 0,
                })
                .await?;
                input_handle.abort();
                break 'run_loop;
            }
        }

        if screen {
            // Leave alternate screen
            print!("\x1b[?1049l");
        }

        self.flush_renderer().await?;
        Ok(RenderState::Stopped)
    }

    async fn update_loop(&'a self) -> Result<()> {
        let mut pending_input = vec![];
        let mut rx = self.input_rx.lock().await;

        loop {
            match rx.try_recv() {
                Ok(InputFrame::Frame(key)) => {
                    pending_input.push(key);
                }
                Ok(InputFrame::Empty) => {}
                Ok(InputFrame::End) => {
                    return Err(eyre::eyre!("input closed!"));
                }
                Err(TryRecvError::Disconnected) => {
                    eprintln!("error: Input disconnected!?");
                    return Err(eyre::eyre!("input disconnected!"));
                }
                Err(TryRecvError::Empty) => {
                    break;
                }
            }
        }

        self.update(&pending_input).await.expect("update failed!");

        Ok(())
    }

    async fn render_loop(
        &'a self,
        last_frame_time: &mut Option<Duration>,
        frame_counter: &mut u128,
        last_fps: &mut f64,
        effective_fps: &mut f64,
        cursor: &Coordinates,
        dimensions: &Dimensions,
    ) -> Result<bool> {
        let start = Instant::now();
        let fps_target = 60;
        let one_second_in_micros = Duration::from_secs(1).as_micros();
        let frame_target = Duration::from_micros((one_second_in_micros as u64) / fps_target);

        let mut render_context = RenderContext {
            last_frame_time: *last_frame_time,
            frame_counter: *frame_counter,
            fps: *last_fps,
            effective_fps: *effective_fps,
            cursor: *cursor,
            dimensions: *dimensions,
            // Default values, these are filled in by the inner render method.
            focus: 0,
        };

        let currently_exiting = match self.render_frame(&mut render_context).await {
            Ok(exiting) => exiting,
            Err(e) => {
                // TODO: Handle gracefully
                eprintln!("Error: {e}");
                return Err(e);
            }
        };

        self.flush_renderer().await?;

        *frame_counter += 1;

        let elapsed = start.elapsed();
        *last_frame_time = Some(elapsed);
        *effective_fps = (one_second_in_micros as f64) / (elapsed.as_micros() as f64);
        *last_fps = if *effective_fps as u64 > fps_target {
            fps_target as f64
        } else {
            *effective_fps
        };

        if let Some(duration) = frame_target.checked_sub(elapsed) {
            tokio::time::sleep(duration).await
        } else {
            // log::warn!("Frame took too long to render: {:?}", elapsed);
        }

        Ok(currently_exiting)
    }

    pub async fn update(&'a self, pending_input: &[Keypress]) -> Result<()> {
        let dimensions = { self.renderer.read().await.dimensions() };
        let mut ui = self.ui.lock().await;
        let exiting = ui.update(pending_input, dimensions).await?;
        if exiting {
            let mut done = self.done.lock().await;
            *done = true;
        }

        Ok(())
    }

    pub async fn render_once(&'a self) -> Result<RenderState> {
        let mut ctx = {
            let renderer = self.renderer.read().await;
            RenderContext {
                last_frame_time: None,
                frame_counter: 0,
                fps: 0f64,
                effective_fps: 0f64,
                cursor: renderer.cursor(),
                dimensions: renderer.dimensions(),
                focus: 0,
            }
        };

        self.render_frame(&mut ctx).await?;

        Ok(RenderState::Running)
    }

    /// Apply any pending `Mailbox`es and render the current frame. Makes no
    /// guarantees about hitting a framerate target, but instead renders as
    /// fast as possible.
    ///
    /// Returns whether or not the UI is currently stopping.
    async fn render_frame(&'a self, ctx: &mut RenderContext) -> Result<bool> {
        let mut ui = self.ui.lock().await;
        let commands = ui.render(ctx).await?;

        let mut renderer = self.renderer.write().await;
        renderer.render(&commands).await?;
        renderer.flush().await?;

        Ok(ui.exiting)
    }

    async fn flush_renderer(&'a self) -> Result<()> {
        let mut renderer = self.renderer.write().await;
        renderer.flush().await?;

        Ok(())
    }

    /// Send a message to the given component.
    pub async fn send(&self, key: Key, message: M) {
        let ui = self.ui.lock().await;
        ui.send(key, message).await;
    }

    /// Send a makeup message to the given component.
    pub async fn send_makeup(&self, key: Key, message: MakeupMessage) {
        let ui = self.ui.lock().await;
        ui.send_makeup(key, message).await;
    }

    /// Send a message to the UI.
    pub async fn send_control(&self, message: UiControlMessage) {
        let ui = self.ui.lock().await;
        ui.send_control(message).await;
    }

    pub async fn move_cursor(&self, x: u64, y: u64) -> Result<()> {
        let mut renderer = self.renderer.write().await;
        renderer.move_cursor(x, y).await?;

        Ok(())
    }

    pub async fn read_at_cursor(&self, count: u64) -> Result<String> {
        let renderer = self.renderer.read().await;
        renderer.read_at_cursor(count).await
    }

    #[cfg(test)]
    pub(crate) fn renderer(&self) -> &RwLocked<Box<dyn Renderer>> {
        &self.renderer
    }

    #[cfg(test)]
    pub(crate) async fn focus(&self) -> Key {
        let ui = self.ui.lock().await;
        ui.focus()
    }
}

#[derive(Derivative)]
#[derivative(Debug)]
struct UI<'a, M: std::fmt::Debug + Send + Sync + Clone> {
    root: Box<dyn Component<Message = M>>,
    post_office: RwLocked<PostOffice<M>>,
    focus: Key,
    exiting: bool,
    #[derivative(Debug = "ignore")]
    taffy: Taffy,
    #[derivative(Debug = "ignore")]
    taffy_lookup: HashMap<Key, Node>,
    _phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a, M: std::fmt::Debug + Send + Sync + Clone + 'static> UI<'a, M> {
    /// Build a new `UI` from the given root component.
    pub(self) fn new(root: Box<dyn Component<Message = M>>) -> Result<Self> {
        let focus_key = root.key();
        let mut taffy = Taffy::new();
        let mut taffy_lookup = HashMap::new();
        Self::build_component_tree(root.as_ref(), &mut taffy, &mut taffy_lookup)?;
        Ok(Self {
            root,
            post_office: Arc::new(RwLock::new(PostOffice::new())),
            focus: focus_key,
            exiting: false,
            taffy,
            taffy_lookup,
            _phantom: std::marker::PhantomData,
        })
    }

    fn build_component_tree(
        root: &dyn Component<Message = M>,
        taffy: &mut Taffy,
        taffy_lookup: &mut HashMap<Key, Node>,
    ) -> Result<()> {
        let (width, height) = root.dimensions()?;
        let root_node = taffy.new_leaf(Style {
            size: Size {
                // TODO: Overflow???
                width: Dimension::Points(width as f32),
                height: Dimension::Points(height as f32),
            },
            ..Default::default()
        })?;
        taffy_lookup.insert(root.key(), root_node);

        if let Some(children) = root.children() {
            for child in children {
                Self::build_component_tree_recursive(root_node, child.as_ref(), taffy)?;
            }
        }

        Ok(())
    }

    fn build_component_tree_recursive(
        parent_node: Node,
        component: &dyn Component<Message = M>,
        taffy: &mut Taffy,
    ) -> Result<()> {
        let (width, height) = component.dimensions()?;
        let node = taffy.new_leaf(Style {
            size: Size {
                // TODO: Overflow???
                width: Dimension::Points(width as f32),
                height: Dimension::Points(height as f32),
            },
            ..Default::default()
        })?;
        taffy.add_child(parent_node, node)?;

        if let Some(children) = component.children() {
            for child in children {
                Self::build_component_tree_recursive(node, child.as_ref(), taffy)?;
            }
        }

        Ok(())
    }

    pub(self) async fn update(
        &mut self,
        pending_input: &[Keypress],
        render_dimensions: Dimensions,
    ) -> Result<bool> {
        let mut post_office = self.post_office.write().await;

        for message in post_office.ui_mailbox() {
            match message {
                UiControlMessage::MoveFocus(key) => {
                    self.focus = *key;
                }
                UiControlMessage::StopRendering => {
                    self.exiting = true;
                }
            }
        }
        post_office.clear_ui_mailbox();

        Self::mail_pending_input(pending_input, &mut post_office, self.focus);
        let taffy_lookup = &mut self.taffy_lookup;
        Self::update_recursive(
            taffy_lookup,
            render_dimensions,
            self.root.as_mut(),
            &mut post_office,
            self.focus,
            self.post_office.clone(),
        )
        .await?;

        Self::build_component_tree(self.root.as_ref(), &mut self.taffy, &mut self.taffy_lookup)?;
        self.taffy.compute_layout(
            *self
                .taffy_lookup
                .get(&self.root.key())
                .expect("root component not found in flex tree!?"),
            Size {
                width: AvailableSpace::Definite(render_dimensions.0 as f32),
                height: AvailableSpace::Definite(render_dimensions.1 as f32),
            },
        )?;

        Ok(self.exiting)
    }

    /// Render the entire UI.
    // TODO: Graceful error handling...
    // TODO: Figure out parallel rendering
    pub(self) async fn render(&mut self, ctx: &mut RenderContext) -> Result<Vec<DrawCommandBatch>> {
        ctx.focus = self.focus;
        let draw_commands = Self::render_recursive(self.root.as_ref(), ctx).await?;
        Ok(draw_commands)
    }

    #[async_recursion]
    async fn render_recursive(
        component: &dyn Component<Message = M>,
        ctx: &RenderContext,
    ) -> Result<Vec<DrawCommandBatch>> {
        let mut draw_commands = vec![];

        draw_commands.push(component.render(ctx).await?);

        if let Some(children) = component.children() {
            for child in children {
                let mut child_draw_commands = Self::render_recursive(child.as_ref(), ctx).await?;
                draw_commands.append(&mut child_draw_commands);
            }
        }

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
        _taffy_lookup: &mut HashMap<Key, Node>,
        render_dimensions: Dimensions,
        component: &mut dyn Component<Message = M>,
        post_office: &mut PostOffice<M>,
        focus: Key,
        post_office_lock: RwLocked<PostOffice<M>>,
    ) -> Result<()> {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

        let mut pending_update = UpdateContext {
            post_office: &mut *post_office,
            sender: MessageSender::new(tx.clone(), focus),
            focus,
            dimensions: render_dimensions,
        };

        // TODO: Update subtrees only
        // let old_keys = Self::get_all_child_keys(component);
        (*component).update(&mut pending_update).await?;
        // let new_keys = Self::get_all_child_keys(component);
        // // TODO: If the old and new component keys don't match, update the taffy tree starting at component.key()'s node
        // if old_keys != new_keys {
        //     for key in old_keys {
        //         // remove from taffy_lookup
        //         taffy_lookup.remove(&key);
        //     }
        //     // TODO: Recursively add all new children to taffy tree
        // }

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

        if let Some(children) = component.children_mut() {
            for child in children {
                Self::update_recursive(
                    _taffy_lookup,
                    render_dimensions,
                    child.as_mut(),
                    post_office,
                    focus,
                    post_office_lock.clone(),
                )
                .await?;
            }
        }

        Ok(())
    }

    #[allow(unused)]
    fn get_all_child_keys(component: &dyn Component<Message = M>) -> HashSet<Key> {
        let mut keys = HashSet::new();
        keys.insert(component.key());

        if let Some(children) = component.children() {
            for child in children {
                keys.extend(Self::get_all_child_keys(child.as_ref()));
            }
        }

        keys
    }

    pub(self) async fn send(&self, key: Key, message: M) {
        let mut post_office = self.post_office.write().await;
        post_office.send(key, message);
    }

    pub(self) async fn send_makeup(&self, key: Key, message: MakeupMessage) {
        let mut post_office = self.post_office.write().await;
        post_office.send_makeup(key, message);
    }

    pub(self) async fn send_control(&self, message: UiControlMessage) {
        let mut post_office = self.post_office.write().await;
        post_office.send_control(message);
    }

    #[cfg(test)]
    pub(self) fn focus(&self) -> Key {
        self.focus
    }
}

#[cfg(test)]
mod tests {
    use crate::component::{DrawCommandBatch, Key, MakeupUpdate, RenderContext};
    use crate::components::EchoText;
    use crate::input::TerminalInput;
    use crate::render::MemoryRenderer;
    use crate::ui::UiControlMessage;
    use crate::{check_mail, Component, Dimensions, DrawCommand, MUI};

    use async_trait::async_trait;
    use eyre::Result;

    #[derive(Debug)]
    struct PingableComponent {
        #[allow(dead_code)]
        state: (),
        key: Key,
        was_pinged: bool,
        children: Vec<Box<dyn Component<Message = PingMessage>>>,
    }

    #[derive(Debug, Clone)]
    enum PingMessage {
        Ping,
    }

    #[async_trait]
    impl Component for PingableComponent {
        type Message = PingMessage;

        fn children(&self) -> Option<Vec<&Box<dyn Component<Message = Self::Message>>>> {
            Some(self.children.iter().collect())
        }

        fn children_mut(
            &mut self,
        ) -> Option<Vec<&mut Box<dyn Component<Message = Self::Message>>>> {
            None
        }

        async fn update(&mut self, ctx: &mut MakeupUpdate<Self>) -> Result<()> {
            use crate::ui::MakeupMessage;
            check_mail!(
                self,
                ctx,
                match _ {
                    MakeupMessage::TextUpdate(_) => {}
                    PingMessage::Ping => {
                        self.was_pinged = true;
                    }
                }
            );

            Ok(())
        }

        async fn render(&self, _ctx: &RenderContext) -> Result<DrawCommandBatch> {
            if !self.was_pinged {
                Ok((self.key, vec![DrawCommand::TextUnderCursor("ping?".into())]))
            } else {
                Ok((self.key, vec![DrawCommand::TextUnderCursor("pong!".into())]))
            }
        }

        fn key(&self) -> Key {
            self.key
        }

        fn dimensions(&self) -> Result<Dimensions> {
            Ok((5, 1))
        }
    }

    #[tokio::test]
    async fn test_messaging_works() -> Result<()> {
        let root = PingableComponent {
            state: (),
            key: crate::component::generate_key(),
            was_pinged: false,
            children: vec![],
        };
        let key = root.key();

        let renderer = MemoryRenderer::new(128, 128);
        let input = TerminalInput::new().await?;
        let ui = MUI::new(Box::new(root), Box::new(renderer), input)?;
        ui.update(&[]).await?;
        ui.render_once().await?;

        {
            let mut renderer = ui.renderer().write().await;
            renderer.move_cursor(0, 0).await?;
            assert_eq!("ping?".to_string(), renderer.read_at_cursor(5).await?);
        }

        ui.send(key, PingMessage::Ping).await;
        ui.update(&[]).await?;
        ui.render_once().await?;

        {
            let mut renderer = ui.renderer().write().await;
            renderer.move_cursor(0, 0).await?;
            assert_eq!("pong!".to_string(), renderer.read_at_cursor(5).await?);
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_ui_messaging_works() -> Result<()> {
        let root = EchoText::<String>::new("blep");
        let key = root.key();

        let renderer = MemoryRenderer::new(128, 128);
        let input = TerminalInput::new().await?;
        let ui = MUI::new(Box::new(root), Box::new(renderer), input)?;
        ui.update(&[]).await?;

        assert_eq!(key, ui.focus().await);

        ui.send_control(UiControlMessage::MoveFocus(0)).await;
        ui.update(&[]).await?;

        assert_eq!(0, ui.focus().await);

        Ok(())
    }
}
