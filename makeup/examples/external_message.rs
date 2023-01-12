use std::sync::Arc;
use std::time::Duration;

use makeup::components::Fps;
use makeup::input::TerminalInput;
use makeup::render::terminal::TerminalRenderer;
use makeup::MUI;

use eyre::Result;
use makeup::ui::UiControlMessage;

#[tokio::main]
async fn main() -> Result<()> {
    let mut root = Fps::new();
    let mut renderer = TerminalRenderer::new();
    let input = TerminalInput::new();
    let mui = Arc::new(MUI::<()>::new(&mut root, &mut renderer, input));
    let stop_mui = mui.clone();

    let mut flag = false;
    'outer: loop {
        if flag {
            break;
        }
        // tokio::select! over the mui.render() future and the time::sleep future

        println!("next");
        tokio::select! {
            _ = tokio::time::sleep(Duration::from_secs(1)) => {
                flag = true;
                stop_mui.send_control(UiControlMessage::StopRendering).await;
            }
            res = mui.render(false) => {
                match res {
                    Ok(_) => {}
                    Err(e) => {
                        eprintln!("Error: {}", e);
                        break 'outer;
                    }
                }
            }
        }
    }

    println!("done");

    Ok(())
}
