//! Extension methods for std mutex for web-safe locking.
//!
//! The web main thread cannot use i32.wait. Using it will give "RuntimeError: Atomics.wait cannot be called in this context".
//!
//! The extension method does spinning lock in main thread, and normal locking in web workers.
//!
//! (TODO Maybe move it to another crate)

#![cfg(feature = "experimental_wasm_multithreading_support")]
#![cfg(target_arch = "wasm32")]

use std::sync::{LockResult, Mutex, MutexGuard, PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard, TryLockError};
use wasm_bindgen::JsCast;
use web_sys::Window;

thread_local! {
    /// Cached result of whether it's on the main thread.
    static IS_MAIN_THREAD: bool = js_sys::global().dyn_into::<Window>().is_ok();
}

/// Returns whether the current thread is the main thread.
/// Result is cached in a thread-local so it's fast.
pub fn is_main_thread() -> bool {
    IS_MAIN_THREAD.with(|&v| v)
}

/// Extension trait for `std::sync::Mutex` that provides web-safe locking.
pub trait MutexWebExt<T> {
    /// Acquires the mutex in a web-safe manner.
    ///
    /// It will do spin locking in main thread (main thread cannot use `i32.wait` instruction).
    ///
    /// In web workers it will do normal locking.
    ///
    /// It's recommended to always lock briefly to avoid main thread spin lock for too long time.
    fn web_safe_lock(&self) -> LockResult<MutexGuard<'_, T>>;
}

impl<T> MutexWebExt<T> for Mutex<T> {
    fn web_safe_lock(&self) -> LockResult<MutexGuard<'_, T>> {
        if is_main_thread() {
            loop {
                match self.try_lock() {
                    Ok(guard) => return Ok(guard),
                    Err(TryLockError::WouldBlock) => {
                        // it's no-op in wasm now. maybe it will be useful in the future.
                        std::hint::spin_loop();
                        continue;
                    }
                    Err(TryLockError::Poisoned(e)) => {
                        return Err(PoisonError::new(e.into_inner()));
                    }
                }
            }
        } else {
            self.lock()
        }
    }
}


pub trait RwLockWebExt<T> {
    fn web_safe_read(&self) -> LockResult<RwLockReadGuard<'_, T>>;

    fn web_safe_write(&self) -> LockResult<RwLockWriteGuard<'_, T>>;
}

impl<T> RwLockWebExt<T> for RwLock<T> {
    fn web_safe_read(&self) -> LockResult<RwLockReadGuard<'_, T>> {
        if is_main_thread() {
            loop {
                match self.try_read() {
                    Ok(guard) => return Ok(guard),
                    Err(TryLockError::WouldBlock) => {
                        std::hint::spin_loop();
                        continue;
                    }
                    Err(TryLockError::Poisoned(e)) => {
                        return Err(PoisonError::new(e.into_inner()));
                    }
                }
            }
        } else {
            self.read()
        }
    }

    fn web_safe_write(&self) -> LockResult<RwLockWriteGuard<'_, T>> {
        if is_main_thread() {
            loop {
                match self.try_write() {
                    Ok(guard) => return Ok(guard),
                    Err(TryLockError::WouldBlock) => {
                        std::hint::spin_loop();
                        continue;
                    }
                    Err(TryLockError::Poisoned(e)) => {
                        return Err(PoisonError::new(e.into_inner()));
                    }
                }
            }
        } else {
            self.write()
        }
    }
}