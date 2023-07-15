use makeup::components::TextInput;
use makeup::input::TerminalInput;
use makeup::render::terminal::TerminalRenderer;
use makeup::MUI;

use eyre::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let mut root = TextInput::new("Type some text here");
    let renderer = TerminalRenderer::new();
    let input = TerminalInput::new();
    let mui = MUI::<()>::new(&mut root, Box::new(renderer), input);
    mui.render(false).await?;

    Ok(())
}
