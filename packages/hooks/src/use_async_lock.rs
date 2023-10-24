use async_std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use dioxus_core::ScopeState;
use std::cell::Cell;
use std::sync::Arc;

pub fn use_async_lock<T: 'static>(
    cx: &ScopeState,
    initialize_rwlock: impl FnOnce() -> T,
) -> &UseAsyncLock<T> {
    let hook = cx.use_hook(|| UseAsyncLock {
        update: cx.schedule_update(),
        value: Arc::new(RwLock::new(initialize_rwlock())),
        dirty: Arc::new(Cell::new(false)),
        gen: 0,
    });

    if hook.dirty.get() {
        hook.gen += 1;
        hook.dirty.set(false);
    }

    hook
}

pub struct UseAsyncLock<T> {
    update: Arc<dyn Fn()>,
    value: Arc<RwLock<T>>,
    dirty: Arc<Cell<bool>>,
    gen: usize,
}

impl<T> Clone for UseAsyncLock<T> {
    fn clone(&self) -> Self {
        Self {
            update: self.update.clone(),
            value: self.value.clone(),
            dirty: self.dirty.clone(),
            gen: self.gen,
        }
    }
}

impl<T> UseAsyncLock<T> {
    pub async fn read(&self) -> RwLockReadGuard<'_, T> {
        self.value.read().await
    }

    pub async fn write(&self) -> RwLockWriteGuard<'_, T> {
        self.needs_update();
        self.value.write().await
    }

    pub async fn set(&self, new: T) {
        *self.value.write().await = new;
        self.needs_update();
    }

    pub async fn write_silent(&self) -> RwLockWriteGuard<'_, T> {
        self.value.write().await
    }

    pub async fn with<O>(&self, immutable_callback: impl FnOnce(&T) -> O) -> O {
        immutable_callback(&*self.read().await)
    }

    pub async fn with_mut<O>(&self, mutable_callback: impl FnOnce(&mut T) -> O) -> O {
        mutable_callback(&mut *self.write().await)
    }

    pub fn needs_update(&self) {
        self.dirty.set(true);
        (self.update)();
    }
}

impl<T> PartialEq for UseAsyncLock<T> {
    fn eq(&self, other: &Self) -> bool {
        if Arc::ptr_eq(&self.value, &other.value) {
            self.gen == other.gen
        } else {
            false
        }
    }
}
