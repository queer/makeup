use makeup::components::{Container, EchoText};
use makeup::input::TerminalInput;
use makeup::render::terminal::TerminalRenderer;
use makeup::MUI;

use eyre::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let mut root = Container::new_with_style(
        vec![
            Box::new(EchoText::new("hello,")),
            Box::new(EchoText::new("world!")),
        ],
        Some(taffy::style::Style {
            flex_direction: taffy::style::FlexDirection::Column,
            ..Default::default()
        }),
    );
    let renderer = TerminalRenderer::new();
    let input = TerminalInput::new().await?;
    let mui = MUI::<()>::new(&mut root, Box::new(renderer), input)?;
    mui.render_once().await?;

    Ok(())
}
