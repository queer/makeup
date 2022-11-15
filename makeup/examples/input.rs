use makeup::components::TextInput;
use makeup::render::terminal::TerminalRenderer;
use makeup::MUI;

use eyre::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let mut root = TextInput::new("Type some text here");
    let mut renderer = TerminalRenderer::new();
    let mui = MUI::<()>::new(&mut root, &mut renderer);
    mui.render(false).await?;

    Ok(())
}
