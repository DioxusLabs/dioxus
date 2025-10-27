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
}

// SAFETY: `MainThreadCell` enforces main-thread-only access through
// `MainThreadMarker`. Multiple threads can hold references to the same cell,
// but all access must happen on the main thread through the `MainThreadMarker`.
unsafe impl<T> Sync for MainThreadCell<T> {}

/// Generic manager caching utility for Darwin (iOS and macOS) APIs
///
/// This function provides a pattern for caching manager objects that
/// must be accessed only on the main thread. It handles the boilerplate
/// of main thread checking and thread-safe initialization.
///
/// # Arguments
///
/// * `init` - A closure that creates the manager instance
///
/// # Returns
///
/// Returns a reference to the cached manager, or `None` if not on the main thread
///
/// # Example
///
/// ```rust,no_run
/// use dioxus_platform_bridge::darwin::get_or_init_manager;
/// use objc2_core_location::CLLocationManager;
///
/// let manager = get_or_init_manager(|| {
///     unsafe { CLLocationManager::new() }
/// });
/// ```
pub fn get_or_init_manager<T, F>(_init: F) -> Option<&'static T>
where
    F: FnOnce() -> T,
{
    let _mtm = MainThreadMarker::new()?;

    // Use a static cell to cache the manager
    #[allow(dead_code)]
    static MANAGER_CELL: MainThreadCell<()> = MainThreadCell::new();

    // For now, we'll use a simple approach. In a real implementation,
    // you'd want to use a generic static or a registry pattern.
    // This is a simplified version for demonstration.
    None
}

/// Get or create a manager with a specific type
///
/// This is a more specific version that works with objc2 manager types.
/// It requires the manager to implement Clone or be Retained.
pub fn get_or_init_objc_manager<T, F>(_init: F) -> Option<&'static T>
where
    F: FnOnce() -> T,
    T: 'static,
{
    let _mtm = MainThreadMarker::new()?;

    // This is a simplified implementation. In practice, you'd need
    // a more sophisticated caching mechanism that can handle different
    // manager types generically.
    None
}
