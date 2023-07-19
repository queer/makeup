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

        let mut data = EchoText::<()>::new(data);

        let ui = {
            use crate::input::TerminalInput;
            use crate::render::TerminalRenderer;
            use crate::MUI;

            let renderer = TerminalRenderer::new();
            let input = TerminalInput::new();
            let ui = MUI::new(&mut data, Box::new(renderer), input);
            ui
        };
        ui.render_once().await?;

        Ok(())
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
    diff: DrawCommandDiff,
}

impl VisualDiff {
    pub fn new(expected: Vec<DrawCommand>, actual: Vec<DrawCommand>) -> Self {
        Self {
            diff: DrawCommandDiff::new(expected, actual),
        }
    }

    pub async fn render(&self) -> Result<()> {
        use crate::render::Renderer;

        if !self.diff.diff.is_empty() {
            async fn read_lines(renderer: &dyn Renderer) -> Vec<String> {
                let mut out = vec![];

                for i in 0..renderer.dimensions().1 {
                    out.push(
                        renderer
                            .read_string(0, i, renderer.dimensions().0 - 1)
                            .await
                            .unwrap()
                            .trim_end()
                            .to_string(),
                    );
                }

                while out.last().map(|s| s.is_empty()).unwrap_or(false) {
                    out.pop();
                }

                out
            }

            let mut expected_renderer = crate::render::MemoryRenderer::new(128, 128);
            let mut actual_renderer = crate::render::MemoryRenderer::new(128, 128);

            expected_renderer
                .render(&[(0, self.diff.expected.clone())])
                .await?;
            actual_renderer
                .render(&[(0, self.diff.actual.clone())])
                .await?;

            let expected_lines = read_lines(&expected_renderer).await;
            let actual_lines = read_lines(&actual_renderer).await;

            let expected_text = expected_lines.join("\n");
            let actual_text = actual_lines.join("\n");

            let mut rendered_diff = String::from("");
            for i in 0..actual_lines.len() {
                use std::fmt::Write;

                if i >= expected_lines.len() {
                    writeln!(
                        &mut rendered_diff,
                        "{}{}",
                        makeup_ansi::Ansi::TerminalBackgroundColour(makeup_ansi::Colour::Red),
                        actual_lines[i]
                    )?;
                } else {
                    // compare char-by-char, marking different characters in red
                    // DO NOT USE .chars() OR .as_bytes()

                    for j in 0..actual_lines[i].len() {
                        if j >= expected_lines[i].len()
                            || actual_lines[i].chars().nth(j).unwrap()
                                != expected_lines[i].chars().nth(j).unwrap()
                        {
                            write!(
                                &mut rendered_diff,
                                "{}{}{}",
                                makeup_ansi::Ansi::Sgr(vec![
                                    makeup_ansi::SgrParameter::HexForegroundColour(0xFF0000)
                                ]),
                                actual_lines[i].chars().nth(j).unwrap(),
                                makeup_ansi::Ansi::Sgr(vec![makeup_ansi::SgrParameter::Reset]),
                            )?;
                        } else {
                            write!(
                                &mut rendered_diff,
                                "{}",
                                actual_lines[i].chars().nth(j).unwrap()
                            )?;
                        }
                    }
                }
            }

            let data = indoc::formatdoc!(
                "\n
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

            let mut data = EchoText::<()>::new(data);

            let ui = {
                use crate::input::TerminalInput;
                use crate::render::TerminalRenderer;
                use crate::MUI;

                let renderer = TerminalRenderer::new();
                let input = TerminalInput::new();
                let ui = MUI::new(&mut data, Box::new(renderer), input);
                ui
            };
            ui.render_once().await?;
        }

        Ok(())
    }
}

impl From<DrawCommandDiff> for VisualDiff {
    fn from(diff: DrawCommandDiff) -> Self {
        Self { diff }
    }
}
