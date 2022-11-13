use std::time::Duration;

use makeup::components::Spinner;
use makeup::render::terminal::TerminalRenderer;
use makeup::MUI;

use eyre::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let mut root = Spinner::new(
        "hello, world!",
        vec!['-', '\\', '|', '/'],
        Duration::from_millis(100),
    );
    let mut renderer = TerminalRenderer::new();
    let mui = MUI::<()>::new(&mut root, &mut renderer);
    mui.render(false).await?;

    Ok(())
}
