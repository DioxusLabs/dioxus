use objc2::MainThreadMarker;
use std::cell::UnsafeCell;

/// A cell that stores values only accessible on the main thread.
///
/// This type is useful for caching singleton-like objects that must only be
/// accessed on the main thread on Darwin platforms (iOS/macOS).
///
/// # Safety
///
/// Access is guarded by requiring a `MainThreadMarker`, ensuring this cell
/// is only touched from the main thread.
///
/// # Example
///
/// ```rust,no_run
/// use dioxus_platform_bridge::darwin::MainThreadCell;
/// use objc2::MainThreadMarker;
///
/// let mtm = MainThreadMarker::new().unwrap();
/// let cell = MainThreadCell::new();
/// let value = cell.get_or_init_with(mtm, || "initialized");
/// ```
pub struct MainThreadCell<T>(UnsafeCell<Option<T>>);

impl<T> MainThreadCell<T> {
    /// Create a new empty cell.
    pub const fn new() -> Self {
        Self(UnsafeCell::new(None))
    }
}

impl<T> Default for MainThreadCell<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> MainThreadCell<T> {
    /// Get or initialize the value in this cell.
    ///
    /// Requires a `MainThreadMarker` to ensure we're on the main thread.
    /// The `init` closure is only called if the cell is currently empty.
    ///
    /// # Panics
    ///
    /// This will panic if the value has not been initialized after calling
    /// the init closure. This should not happen in practice but is a safety
    /// check to ensure thread safety.
    pub fn get_or_init_with<F>(&self, _mtm: MainThreadMarker, init: F) -> &T
    where
        F: FnOnce() -> T,
    {
        // SAFETY: Access is guarded by requiring a `MainThreadMarker`, so this
        // is only touched from the main thread.
        unsafe {
            let slot = &mut *self.0.get();
            if slot.is_none() {
                *slot = Some(init());
            }
            slot.as_ref().expect("Manager initialized")
        }
    }

    /// Fallible variant of [`get_or_init_with`] that allows returning an error during initialization.
    pub fn get_or_try_init_with<F, E>(&self, _mtm: MainThreadMarker, init: F) -> Result<&T, E>
    where
        F: FnOnce() -> Result<T, E>,
    {
        unsafe {
            let slot = &mut *self.0.get();
            if slot.is_none() {
                *slot = Some(init()?);
            }
            Ok(slot.as_ref().expect("Manager initialized"))
        }
    }
}

// SAFETY: `MainThreadCell` enforces main-thread-only access through
// `MainThreadMarker`. Multiple threads can hold references to the same cell,
// but all access must happen on the main thread through the `MainThreadMarker`.
unsafe impl<T> Sync for MainThreadCell<T> {}
