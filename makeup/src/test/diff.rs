use std::fmt::Display;

use crate::components::EchoText;
use crate::DrawCommand;

use eyre::Result;

pub struct DrawCommandDiff {
    pub expected: Vec<DrawCommand>,
    pub actual: Vec<DrawCommand>,

    pub diff: Vec<DiffLine>,
}

impl DrawCommandDiff {
    pub fn new(expected: Vec<DrawCommand>, actual: Vec<DrawCommand>) -> Self {
        let mut diff = vec![];

        let mut expected_iter = expected.iter();
        let mut actual_iter = actual.iter();

        let mut line_number = 0;

        loop {
            let expected = expected_iter.next();
            let actual = actual_iter.next();

            if expected.is_none() && actual.is_none() {
                break;
            }

            let expected = expected.map(|c| Some(c.clone())).unwrap_or_default();
            let actual = actual.map(|c| Some(c.clone())).unwrap_or_default();

            diff.push(DiffLine {
                line_number,
                different: expected != actual,
                expected,
                actual,
            });

            line_number += 1;
        }

        Self {
            expected,
            actual,
            diff,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.diff.is_empty()
    }

    pub async fn render(&self) -> Result<()> {
        let mut data = String::from("error rendering test ui!\n\n----------------\n\n");

        for line in &self.diff {
            let colour = if line.different {
                makeup_ansi::Ansi::Sgr(vec![makeup_ansi::SgrParameter::HexForegroundColour(
                    0xFF0000,
                )])
            } else {
                makeup_ansi::Ansi::Sgr(vec![makeup_ansi::SgrParameter::Reset])
            };

            data.push_str(&format!(
                "{colour}{line}{}",
                makeup_ansi::Ansi::Sgr(vec![makeup_ansi::SgrParameter::Reset])
            ));
        }

        let data = EchoText::<()>::new(data);

        let ui = {
            use crate::input::TerminalInput;
            use crate::render::TerminalRenderer;
            use crate::MUI;

            let renderer = TerminalRenderer::new();
            let input = TerminalInput::new().await?;
            let ui = MUI::new(Box::new(data), Box::new(renderer), input);
            ui
        };
        ui.render_once().await?;

        Ok(())
    }

    pub async fn into_visual_diff(&self) -> Result<VisualDiff> {
        VisualDiff::new(self).await
    }
}

#[derive(Debug)]
pub struct DiffLine {
    pub line_number: usize,
    pub expected: Option<DrawCommand>,
    pub actual: Option<DrawCommand>,
    pub different: bool,
}

impl Display for DiffLine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "line {}: expected: {:?}, actual: {:?}",
            self.line_number, self.expected, self.actual
        )
    }
}

pub struct VisualDiff {
    rendered_diff: String,
    is_different: bool,
}

impl VisualDiff {
    pub async fn new(diff: &DrawCommandDiff) -> Result<Self> {
        use crate::render::Renderer;

        async fn read_lines(renderer: &dyn Renderer) -> Vec<String> {
            let mut out = vec![];

            for i in 0..renderer.dimensions().1 {
                let line = renderer
                    .read_string(0, i, renderer.dimensions().0 - 1)
                    .await
                    .unwrap()
                    .trim_end()
                    .to_string();
                out.push(line);
            }

            while out.last().map(|s| s.is_empty()).unwrap_or(false) {
                out.pop();
            }

            out
        }

        let mut expected_renderer = crate::render::MemoryRenderer::new(128, 128);
        let mut actual_renderer = crate::render::MemoryRenderer::new(128, 128);

        expected_renderer
            .render(&[(0, diff.expected.clone())])
            .await?;
        actual_renderer.render(&[(0, diff.actual.clone())]).await?;

        let expected_lines = read_lines(&expected_renderer).await;
        let actual_lines = read_lines(&actual_renderer).await;

        let expected_text = expected_lines.join("\n");
        let actual_text = actual_lines.join("\n");

        let mut rendered_diff = String::from("");
        for i in 0..actual_lines.len() {
            use std::fmt::Write;

            if i >= expected_lines.len() {
                write!(
                    &mut rendered_diff,
                    "{}{}{}",
                    makeup_ansi::Ansi::Sgr(vec![makeup_ansi::SgrParameter::HexBackgroundColour(
                        0xFF0000
                    )]),
                    actual_lines[i],
                    makeup_ansi::Ansi::Sgr(vec![makeup_ansi::SgrParameter::Reset]),
                )?;
            } else {
                let mut expected_chars = expected_lines[i].chars();
                let mut actual_chars = actual_lines[i].chars();

                // for each character in the actual line, find each range of characters
                // that is different
                // store them in a Vec<(start, end)>
                let mut different_ranges = vec![];
                let mut start = 0;
                let mut end = 0;
                let mut different = false;
                loop {
                    let expected = expected_chars.next();
                    let actual = actual_chars.next();

                    if expected.is_none() && actual.is_none() {
                        break;
                    }

                    if expected != actual {
                        if !different {
                            start = end;
                            different = true;
                        }
                    } else if different {
                        different_ranges.push((start, end));
                        different = false;
                    }

                    end += 1;
                }

                if different {
                    different_ranges.push((start, end));
                }

                // for each range, mark red
                let actual_chars: Vec<char> = actual_lines[i].chars().collect();
                let mut last_position = 0;
                for range in different_ranges {
                    if range.0 >= actual_chars.len() {
                        // If the range exists outside of the actual line, then
                        // we need to render red past the end of the line but
                        // without any actual text
                        let padding = " ".repeat(range.1 - range.0);
                        let up_to_range: String =
                            actual_chars[0..actual_chars.len()].iter().collect();
                        last_position = actual_chars.len();
                        write!(
                            &mut rendered_diff,
                            "{reset}{up_to_range}{red}{padding}{reset}",
                            red = makeup_ansi::Ansi::Sgr(vec![
                                makeup_ansi::SgrParameter::HexBackgroundColour(0xFF0000)
                            ]),
                            reset = makeup_ansi::Ansi::Sgr(vec![makeup_ansi::SgrParameter::Reset]),
                        )?;
                    } else {
                        let up_to_range: String =
                            actual_chars[last_position..range.0].iter().collect();
                        last_position = std::cmp::min(range.1, actual_chars.len());

                        let padding = if last_position < range.1 {
                            " ".repeat(range.1 - last_position)
                        } else {
                            String::new()
                        };

                        let range: String = actual_chars[range.0..last_position].iter().collect();

                        write!(
                            &mut rendered_diff,
                            "{reset}{up_to_range}{red}{range}{padding}{reset}",
                            reset = makeup_ansi::Ansi::Sgr(vec![makeup_ansi::SgrParameter::Reset]),
                            red = makeup_ansi::Ansi::Sgr(vec![
                                makeup_ansi::SgrParameter::HexBackgroundColour(0xFF0000)
                            ]),
                        )?;
                    }
                }

                let up_to_range: String = actual_chars[last_position..].iter().collect();
                write!(
                    &mut rendered_diff,
                    "{}{}",
                    makeup_ansi::Ansi::Sgr(vec![makeup_ansi::SgrParameter::Reset]),
                    up_to_range,
                )?;
            }
            writeln!(
                &mut rendered_diff,
                "{}",
                makeup_ansi::Ansi::Sgr(vec![makeup_ansi::SgrParameter::Reset])
            )?;
        }

        let rendered_diff = rendered_diff.trim_end();

        let data = indoc::formatdoc!(
            "test ui did not match expected output!!!

            visual diff:

            ----------------

            expected:

            {expected_text}

            ----------------

            actual:

            {actual_text}

            ----------------

            diff:

            {rendered_diff}

            ----------------
            ",
        );

        Ok(Self {
            rendered_diff: data,
            is_different: expected_text != actual_text,
        })
    }

    pub async fn render(&self) -> Result<()> {
        if self.is_different {
            let data = EchoText::<()>::new(&self.rendered_diff);

            let ui = {
                use crate::input::TerminalInput;
                use crate::render::TerminalRenderer;
                use crate::MUI;

                let renderer = TerminalRenderer::new();
                let input = TerminalInput::new().await?;
                let ui = MUI::new(Box::new(data), Box::new(renderer), input);
                ui
            };
            ui.render_once().await?;
        }

        Ok(())
    }

    pub fn is_different(&self) -> bool {
        self.is_different
    }
}

#[cfg(test)]
mod tests {
    use async_trait::async_trait;
    use eyre::Result;

    use crate::component::{DrawCommandBatch, Key, MakeupUpdate, RenderContext};
    use crate::test::{assert_renders_many, static_text};
    use crate::ui::RwLocked;
    use crate::{Component, Dimensions, DrawCommand};

    #[derive(Debug)]
    struct LinesComponent<'a> {
        #[allow(dead_code)]
        state: (),
        children: Vec<RwLocked<&'a mut dyn Component<Message = ()>>>,
        key: Key,
    }

    #[async_trait]
    impl<'a> Component for LinesComponent<'a> {
        type Message = ();

        fn children(&self) -> Option<Vec<&dyn Component<Message = Self::Message>>> {
            None
        }

        async fn update(&mut self, _ctx: &mut MakeupUpdate<Self>) -> Result<()> {
            Ok(())
        }

        async fn render(&self, _ctx: &RenderContext) -> Result<DrawCommandBatch> {
            Ok((
                self.key,
                vec![
                    DrawCommand::TextUnderCursor("line 1    \n".into()),
                    DrawCommand::TextUnderCursor("lime 2\n".into()),
                    DrawCommand::TextUnderCursor("line 3\n".into()),
                    DrawCommand::TextUnderCursor("line 4\n".into()),
                    DrawCommand::TextUnderCursor("line 5\n".into()),
                ],
            ))
        }

        async fn update_pass(&mut self, _ctx: &mut MakeupUpdate<Self>) -> Result<()> {
            Ok(())
        }

        async fn render_pass(&self, ctx: &RenderContext) -> Result<Vec<DrawCommandBatch>> {
            let mut out = vec![];
            let render = self.render(ctx).await?;
            out.push(render);

            for child in &self.children {
                let child = child.read().await;
                let mut render = child.render_pass(ctx).await?;
                out.append(&mut render);
            }

            Ok(out)
        }

        fn key(&self) -> Key {
            self.key
        }

        fn dimensions(&self) -> Result<Dimensions> {
            unimplemented!()
        }
    }

    #[tokio::test]
    #[should_panic]
    async fn test_diff_works() {
        async fn __do_test() -> Result<()> {
            let root = LinesComponent {
                state: (),
                children: vec![],
                key: crate::component::generate_key(),
            };

            assert_renders_many!(
                vec![
                    static_text!("line 1\n"),
                    static_text!("line 2\n"),
                    static_text!("line 3\n"),
                ],
                root
            );

            Ok(())
        }

        __do_test().await.unwrap();
    }
}
