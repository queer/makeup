use makeup::components::EchoText;
use makeup::input::TerminalInput;
use makeup::render::terminal::TerminalRenderer;
use makeup::MUI;

use eyre::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let mut root = EchoText::new("hello, world!");
    let renderer = TerminalRenderer::new();
    let input = TerminalInput::new();
    let mui = MUI::<()>::new(&mut root, Box::new(renderer), input);
    mui.render_once().await?;

    Ok(())
}
