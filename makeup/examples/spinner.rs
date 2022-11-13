use makeup::components::Spinner;
use makeup::render::terminal::TerminalRenderer;
use makeup::MUI;

use eyre::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let mut root = Spinner::new("hello, world!", vec!['-', '\\', '|', '/']);
    let mut renderer = TerminalRenderer::new(128, 128);
    let mui = MUI::<()>::new(&mut root, &mut renderer);
    mui.render().await?;

    Ok(())
}
