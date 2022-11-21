use std::time::Duration;

use async_trait::async_trait;
use colorgrad::Gradient;
use either::Either;
use makeup::component::{
    DrawCommandBatch, ExtractMessageFromComponent, Key, MakeupMessage, RenderContext, UpdateContext,
};
use makeup::input::TerminalInput;
use makeup::render::terminal::TerminalRenderer;
use makeup::{Ansi, Component, DrawCommand, LineEraseMode, SgrParameter, MUI};

use eyre::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let gradient = colorgrad::CustomGradient::new()
        .html_colors(&[
            "#FFB3BA", "#FFDFBA", "#FFFFBA", "#BAFFC9", "#BAE1FF", "#D0BAFF", "#FFBAF2", "#FFB3BA",
        ])
        .build()?;
    let mut root = Wave::new(gradient);
    let mut renderer = TerminalRenderer::new();
    let input = TerminalInput::new();
    let mui = MUI::new(&mut root, &mut renderer, input);
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

    fn children(&self) -> Option<Vec<&dyn Component<Message = Self::Message>>> {
        None
    }

    async fn update(
        &mut self,
        ctx: &mut UpdateContext<ExtractMessageFromComponent<Self>>,
    ) -> Result<()> {
        let sender = ctx.sender.clone();
        if !self.started {
            self.started = true;
            sender.send_makeup_message(self.key(), MakeupMessage::TimerTick(DURATION))?;
        }

        if let Some(mailbox) = ctx.post_office.mailbox(self) {
            for msg in mailbox.iter() {
                match msg {
                    Either::Left(_msg) => {
                        // log::debug!("Spinner received message: {:?}", msg);
                    }
                    #[allow(clippy::single_match)]
                    Either::Right(msg) => match msg {
                        MakeupMessage::TimerTick(_) => {
                            self.step = (self.step + 1) % 10;
                            let key = self.key();
                            let sender = ctx.sender.clone();

                            tokio::spawn(async move {
                                tokio::time::sleep(DURATION).await;
                                sender
                                    .send_makeup_message(key, MakeupMessage::TimerTick(DURATION))
                                    .unwrap();
                            });
                        }
                        _ => {}
                    },
                }
            }
            mailbox.clear();
        }

        Ok(())
    }

    async fn render(&self, ctx: &RenderContext) -> Result<DrawCommandBatch> {
        let mut commands = vec![];

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
            "{}fps ({:.2}fps effective), dimensions {:?}",
            ctx.fps, ctx.effective_fps, ctx.dimensions
        )));

        self.batch(commands)
    }

    async fn update_pass(
        &mut self,
        ctx: &mut UpdateContext<ExtractMessageFromComponent<Self>>,
    ) -> Result<()> {
        self.update(ctx).await
    }

    async fn render_pass(&self, ctx: &RenderContext) -> Result<Vec<DrawCommandBatch>> {
        Ok(vec![self.render(ctx).await?])
    }

    fn key(&self) -> Key {
        self.key
    }
}
