use makeup::components::EchoText;
use makeup::input::TerminalInput;
use makeup::render::terminal::TerminalRenderer;
use makeup::MUI;

use eyre::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let root = EchoText::new("hello, world!");
    let renderer = TerminalRenderer::new();
    let input = TerminalInput::new().await?;
    let mui = MUI::<()>::new(Box::new(root), Box::new(renderer), input)?;
    mui.render_once().await?;

    Ok(())
}
