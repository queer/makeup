#[macro_export]
macro_rules! static_text {
    ($text:expr) => {
        $crate::DrawCommand::TextUnderCursor($text.into())
    };
}

#[macro_export]
macro_rules! assert_renders_many {
    // (expected, component)
    // create a fake context and apply it to the component
    ($expected:expr, $component:expr) => {{
        let ctx = $crate::test::fake_render_ctx();
        let (_key, actual) = $component.render(&ctx).await?;

        if $expected != actual {
            use $crate::render::Renderer;

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

            let mut expected_renderer = $crate::render::MemoryRenderer::new(128, 128);
            let mut actual_renderer = $crate::render::MemoryRenderer::new(128, 128);

            expected_renderer
                .render(&[($component.key(), $expected)])
                .await?;
            actual_renderer
                .render(&[($component.key(), actual)])
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
                        if j >= expected_lines[i].len() {
                            write!(
                                &mut rendered_diff,
                                "{}{}{}",
                                makeup_ansi::Ansi::Sgr(vec![
                                    makeup_ansi::SgrParameter::HexForegroundColour(0xFF0000)
                                ]),
                                actual_lines[i].chars().nth(j).unwrap(),
                                makeup_ansi::Ansi::Sgr(vec![makeup_ansi::SgrParameter::Reset]),
                            )?;
                        } else if actual_lines[i].chars().nth(j).unwrap()
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

            let panic = indoc::formatdoc!(
                "\n
                ----
                error rendering test ui!

                expected:

                {expected_text}

                actual:

                {actual_text}

                diff:

                {rendered_diff}
                ",
            );

            panic!("{}", panic);
        }
    }};
}

#[macro_export]
macro_rules! assert_renders_one {
    // (expected, component)
    // create a fake context and apply it to the component
    ($expected:expr, $component:expr) => {{
        $crate::assert_renders_many!(vec![$expected], $component);
    }};
}

#[macro_export]
macro_rules! make_test_ui {
    ($root:expr) => {{
        let renderer = MemoryRenderer::new(128, 128);
        let input = TerminalInput::new();
        let ui = MUI::new(&mut $root, Box::new(renderer), input);
        ui
    }};
}

#[doc(hidden)]
pub fn fake_render_ctx() -> crate::component::RenderContext {
    crate::component::RenderContext {
        last_frame_time: None,
        frame_counter: 0,
        fps: 0f64,
        effective_fps: 0f64,
        cursor: (0, 0),
        dimensions: (0, 0),
        focus: 0,
    }
}
