use std::time::Duration;

use makeup::components::Spinner;
use makeup::input::TerminalInput;
use makeup::render::terminal::TerminalRenderer;
use makeup::MUI;

use eyre::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let mut root = Spinner::<()>::new(
        "hello, world!",
        vec!['-', '\\', '|', '/'],
        Duration::from_millis(100),
    );
    let renderer = TerminalRenderer::new();
    let input = TerminalInput::new().await?;
    let mui = MUI::<()>::new(&mut root, Box::new(renderer), input);
    mui.render(false).await?;

    Ok(())
}
