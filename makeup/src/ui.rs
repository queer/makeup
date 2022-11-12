use async_recursion::async_recursion;
use dashmap::DashMap;
use eyre::Result;
use futures_util::{stream::FuturesUnordered, StreamExt};

use crate::{Component, DrawCommand};

pub struct UI<'a, M: std::fmt::Debug + Send + Sync> {
    root: &'a mut dyn Component<'a, Message = M>,
    #[doc(hidden)]
    _phantom: std::marker::PhantomData<&'a M>,
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

        if let Some(children) = component.children_mut() {
            let ordered_child_keys = children.iter().map(|x| x.key()).collect::<Vec<&'a str>>();

            let results = Self::parallel_render(children).await?;

            for key in ordered_child_keys {
                if let Some(commands) = results.get(key).take() {
                    for command in commands.iter() {
                        draw_commands.push(command.clone());
                    }
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
