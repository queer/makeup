use async_recursion::async_recursion;
#[allow(unused)]
#[deny(unsafe_code)]
use async_trait::async_trait;
use dashmap::DashMap;
use eyre::Result;
use futures_util::stream::FuturesUnordered;
use futures_util::StreamExt;

pub struct UI<'a, M: std::fmt::Debug + Send + Sync> {
    root: &'a mut dyn Component<'a, Message = M>,
    #[doc(hidden)]
    _phantom: std::marker::PhantomData<&'a M>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum DrawCommand {
    TextUnderCursor(String),
    TextAt { x: usize, y: usize, text: String },
}

impl<'a, M: std::fmt::Debug + Send + Sync> UI<'a, M> {
    /// Build a new `UI` from the given root component.
    pub fn new(root: &'a mut dyn Component<'a, Message = M>) -> Self {
        Self {
            root,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Render the entire UI.
    // TODO: Graceful error handling...
    pub async fn render(&'a mut self) -> Result<Vec<DrawCommand>> {
        let (_key, draw_commands) = Self::render_recursive(self.root).await?;
        Ok(draw_commands)
    }

    #[async_recursion]
    async fn render_recursive(
        component: &'a mut dyn Component<'a, Message = M>,
    ) -> Result<(&'a str, Vec<DrawCommand>)> {
        let key = component.key();
        let mut draw_commands: Vec<DrawCommand> = Vec::new();

        for x in component.render().await? {
            draw_commands.push(x);
        }

        let children = component.children_mut();
        let ordered_child_keys = children
            .iter()
            .map(|x| x.key())
            .collect::<Vec<&'a str>>();

        let results = Self::parallel_render(children).await?;

        for key in ordered_child_keys {
            if let Some(commands) = results.get(key).take() {
                for command in commands.iter() {
                    draw_commands.push(command.clone());
                }
            }
        }

        Ok((key, draw_commands))
    }

    async fn parallel_render(
        components: &'a mut [&mut dyn Component<'a, Message = M>],
    ) -> Result<DashMap<&'a str, Vec<DrawCommand>>> {
        let results = DashMap::new();

        let mut child_render_futures = FuturesUnordered::new();
        for child in components.iter_mut() {
            child_render_futures.push(Self::render_recursive(&mut **child));
        }

        while let Some(render_result) = child_render_futures.next().await {
            let (key, draw_commands) = render_result?;
            results.insert(key, draw_commands);
        }

        Ok(results)
    }

    /// A read-only view of the UI component tree.
    pub fn tree_view(&'a mut self) -> &'a mut dyn Component<'a, Message = M> {
        self.root
    }
}

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

#[cfg(test)]
mod tests {
    use super::{Component, DrawCommand, UI};

    use async_trait::async_trait;
    use eyre::Result;

    #[derive(Debug)]
    struct BasicComponent<'a> {
        state: (),
        children: Vec<&'a mut dyn Component<'a, Message = ()>>,
    }

    #[async_trait]
    impl<'a> Component<'a> for BasicComponent<'a> {
        type Message = ();

        async fn render(&self) -> Result<Vec<DrawCommand>> {
            Ok(vec![DrawCommand::TextUnderCursor(
                "henol world".to_string(),
            )])
        }

        async fn on_message(&mut self, _message: Self::Message) -> Result<Option<Self::Message>> {
            Ok(Some(()))
        }

        fn children(&'a self) -> Vec<&dyn Component<'a, Message = Self::Message>> {
            self.children.iter().map(|c| &**c).collect()
        }

        fn children_mut(
            &'a mut self,
        ) -> &'a mut Vec<&'a mut dyn Component<'a, Message = Self::Message>> {
            &mut self.children
        }

        fn key(&self) -> &'a str {
            "basic"
        }
    }

    #[tokio::test]
    async fn test_it_works() -> Result<()> {
        let mut root = BasicComponent {
            state: (),
            children: vec![],
        };

        let mut ui = UI::new(&mut root);

        assert_eq!(
            vec![DrawCommand::TextUnderCursor("henol world".to_string(),)].as_slice(),
            ui.render().await?.as_slice(),
        );

        Ok(())
    }
}
