use std::sync::Arc;

use tokio::sync::RwLock;

/// Helper struct to make dealing with an `Arc<RwLock<T>>` easier.
#[derive(Debug)]
pub struct RwLocked<T> {
    inner: Arc<RwLock<T>>,
}

impl<T> Clone for RwLocked<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T> RwLocked<T> {
    pub fn new(inner: T) -> Self {
        Self {
            inner: Arc::new(RwLock::new(inner)),
        }
    }

    pub async fn read(&self) -> tokio::sync::RwLockReadGuard<'_, T> {
        self.inner.read().await
    }

    pub async fn write(&self) -> tokio::sync::RwLockWriteGuard<'_, T> {
        self.inner.write().await
    }
}

/// Downcsat any type into [`Any`].
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
