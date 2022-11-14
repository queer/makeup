use makeup::components::EchoText;
use makeup::render::terminal::TerminalRenderer;
use makeup::MUI;

use eyre::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let mut root = EchoText::new("hello, world!");
    let mut renderer = TerminalRenderer::new();
    let mui = MUI::<()>::new(&mut root, &mut renderer);
    mui.render_once().await?;

    Ok(())
}
