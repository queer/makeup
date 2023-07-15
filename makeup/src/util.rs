/// Downcast any type into [`std::any::Any`].
pub trait AsAny {
    fn as_any(&self) -> &dyn std::any::Any;

    fn as_mut_any(&mut self) -> &mut dyn std::any::Any;
}

impl<T: std::any::Any> AsAny for T {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_mut_any(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
