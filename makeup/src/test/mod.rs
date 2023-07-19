pub mod diff;

#[doc(hidden)]
#[macro_export]
macro_rules! __THIS_IS_NOT_PUBLIC_DO_NOT_CALL_static_text {
    ($text:expr) => {
        $crate::DrawCommand::TextUnderCursor($text.into())
    };
}

#[doc(inline)]
pub use __THIS_IS_NOT_PUBLIC_DO_NOT_CALL_static_text as static_text;

#[doc(hidden)]
#[macro_export]
macro_rules! __THIS_IS_NOT_PUBLIC_DO_NOT_CALL_assert_renders_many {
    // (expected, component)
    // create a fake context and apply it to the component
    ($expected:expr, $component:expr) => {{
        let ctx = $crate::test::fake_render_ctx();
        let (_key, actual) = $component.render(&ctx).await?;

        let diff = $crate::test::diff::DrawCommandDiff::new($expected, actual);

        if !diff.is_empty() {
            diff.render().await?;
            let diff: $crate::test::diff::VisualDiff = diff.into();
            diff.render().await?;
        }
    }};
}

#[doc(inline)]
pub use __THIS_IS_NOT_PUBLIC_DO_NOT_CALL_assert_renders_many as assert_renders_many;

#[doc(hidden)]
#[macro_export]
macro_rules! __THIS_IS_NOT_PUBLIC_DO_NOT_CALL_assert_renders_one {
    // (expected, component)
    // create a fake context and apply it to the component
    ($expected:expr, $component:expr) => {{
        $crate::test::assert_renders_many!(vec![$expected], $component);
    }};
}

#[doc(inline)]
pub use __THIS_IS_NOT_PUBLIC_DO_NOT_CALL_assert_renders_one as assert_renders_one;

#[doc(hidden)]
#[macro_export]
macro_rules! __THIS_IS_NOT_PUBLIC_DO_NOT_CALL_make_test_ui {
    ($root:expr) => {{
        use $crate::input::TerminalInput;
        use $crate::render::MemoryRenderer;
        use $crate::MUI;

        let renderer = MemoryRenderer::new(128, 128);
        let input = TerminalInput::new();
        let ui = MUI::new(&mut $root, Box::new(renderer), input);
        ui
    }};
}

#[doc(inline)]
pub use __THIS_IS_NOT_PUBLIC_DO_NOT_CALL_make_test_ui as make_test_ui;

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
