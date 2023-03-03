use std::sync::Arc;
use std::time::Duration;

use makeup::components::Fps;
use makeup::input::TerminalInput;
use makeup::render::terminal::TerminalRenderer;
use makeup::MUI;

use eyre::Result;
use makeup::ui::{RenderState, UiControlMessage};

#[tokio::main]
async fn main() -> Result<()> {
    let mut root = Fps::new();
    let mut renderer = TerminalRenderer::new();
    let input = TerminalInput::new();
    let mui = Arc::new(MUI::<()>::new(&mut root, &mut renderer, input));
    let stop_mui = mui.clone();

    'outer: loop {
        // tokio::select! over the mui.render() future and the time::sleep future
        tokio::select! {
            _ = tokio::time::sleep(Duration::from_secs(1)) => {
                stop_mui.send_control(UiControlMessage::StopRendering).await;
            }
            res = mui.render(false) => {
                match res {
                    Ok(RenderState::Stopped) => {
                        break 'outer;
                    }
                    Ok(_) => {}
                    Err(e) => {
                        eprintln!("Error: {e}");
                        break 'outer;
                    }
                }
            }
        }
    }

    Ok(())
}
