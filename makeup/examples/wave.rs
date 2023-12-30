use std::time::Duration;

use async_trait::async_trait;
use colorgrad::Gradient;
use makeup::component::{DrawCommandBatch, Key, MakeupMessage, MakeupUpdate, RenderContext};
use makeup::input::TerminalInput;
use makeup::render::terminal::TerminalRenderer;
use makeup::{check_mail, Ansi, Component, DrawCommand, LineEraseMode, SgrParameter, MUI};

use eyre::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let gradient = colorgrad::CustomGradient::new()
        .html_colors(&[
            "#FFB3BA", "#FFDFBA", "#FFFFBA", "#BAFFC9", "#BAE1FF", "#D0BAFF", "#FFBAF2", "#FFB3BA",
        ])
        .build()?;
    let root = Wave::new(gradient);
    let renderer = TerminalRenderer::new();
    let input = TerminalInput::new().await?;
    let mui = MUI::new(Box::new(root), Box::new(renderer), input)?;
    mui.render(false).await?;

    Ok(())
}

const DURATION: Duration = Duration::from_millis(32);

#[derive(Debug)]
struct Wave {
    key: Key,
    gradient: Gradient,
    step: u64,
    started: bool,
}

impl Wave {
    fn new(gradient: Gradient) -> Wave {
        Wave {
            key: makeup::component::generate_key(),
            gradient,
            step: 0,
            started: false,
        }
    }
}

#[async_trait]
impl Component for Wave {
    type Message = ();

    fn children(&self) -> Option<Vec<&Box<dyn Component<Message = Self::Message>>>> {
        None
    }

    fn children_mut(&mut self) -> Option<Vec<&mut Box<dyn Component<Message = Self::Message>>>> {
        None
    }

    async fn update(&mut self, ctx: &mut MakeupUpdate<Self>) -> Result<()> {
        let sender = ctx.sender.clone();
        if !self.started {
            self.started = true;
            sender.send_makeup_message(self.key(), MakeupMessage::TimerTick(DURATION))?;
        }

        check_mail!(
            self,
            ctx,
            match _ {
                MakeupMessage::TimerTick(_) => {
                    self.step = (self.step + 1) % 10;
                    ctx.sender.send_makeup_message_after(
                        self.key(),
                        MakeupMessage::TimerTick(DURATION),
                        DURATION,
                    )?;
                }
            }
        );

        Ok(())
    }

    async fn render(&self, ctx: &RenderContext) -> Result<DrawCommandBatch> {
        let mut commands = vec![];

        commands.push(DrawCommand::HideCursor);

        let mut colours = self.gradient.colors(ctx.dimensions.1 as usize - 1);
        let len = &colours.len();
        colours.rotate_right(self.step as usize * (len / 10));

        for (i, colour) in colours.iter().enumerate() {
            let [r, g, b, _] = colour.to_rgba8();
            let r = r as u32;
            let g = g as u32;
            let b = b as u32;
            commands.push(DrawCommand::TextAt {
                x: 0,
                y: i as u64,
                text: format!(
                    "{}{}\n",
                    Ansi::Sgr(vec![SgrParameter::HexForegroundColour(
                        r << 16 | g << 8 | b
                    )]),
                    "█".repeat(ctx.dimensions.0 as usize)
                ),
            });
        }

        commands.push(DrawCommand::EraseCurrentLine(
            LineEraseMode::FromCursorToEnd,
        ));
        commands.push(DrawCommand::TextUnderCursor(format!(
            "{:.2}fps ({:.2}fps effective), dimensions {:?}",
            ctx.fps, ctx.effective_fps, ctx.dimensions
        )));

        commands.push(DrawCommand::ShowCursor);

        self.batch(commands)
    }

    fn key(&self) -> Key {
        self.key
    }

    fn dimensions(&self) -> Result<(u64, u64)> {
        Ok((0, 0))
    }
}
