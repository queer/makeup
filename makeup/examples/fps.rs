use makeup::components::Fps;
use makeup::input::TerminalInput;
use makeup::render::terminal::TerminalRenderer;
use makeup::MUI;

use eyre::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let mut root = Fps::new();
    let renderer = TerminalRenderer::new();
    let input = TerminalInput::new().await?;
    let mui = MUI::<()>::new(&mut root, Box::new(renderer), input)?;
    mui.render(false).await?;

    Ok(())
}
