#[macro_export]
macro_rules! static_text {
    ($text:expr) => {
        $crate::DrawCommand::TextUnderCursor($text.into())
    };
}

#[macro_export]
macro_rules! assert_renders_one {
    // (expected, component)
    // create a fake context and apply it to the component
    ($expected:expr, $component:expr) => {{
        let ctx = $crate::test::fake_render_ctx();
        let (_key, actual) = $component.render(&ctx).await?;
        assert_eq!(vec![$expected], actual);
    }};
}

#[macro_export]
macro_rules! assert_renders_many {
    // (expected, component)
    // create a fake context and apply it to the component
    ($expected:expr, $component:expr) => {{
        let ctx = $crate::test::fake_render_ctx();
        let (_key, actual) = $component.render(&ctx).await?;
        assert_eq!($expected, actual);
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
