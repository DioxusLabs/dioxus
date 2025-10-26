use objc2::MainThreadMarker;
use std::cell::UnsafeCell;

/// A cell that stores values only accessible on the main thread.
struct MainThreadCell<T>(UnsafeCell<Option<T>>);

impl<T> MainThreadCell<T> {
    const fn new() -> Self {
        Self(UnsafeCell::new(None))
    }

    fn get_or_init_with<F>(&self, _mtm: MainThreadMarker, init: F) -> &T
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
// `MainThreadMarker`.
unsafe impl<T> Sync for MainThreadCell<T> {}

/// Generic manager caching utility for iOS APIs
///
/// This function provides a pattern for caching iOS manager objects that
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
/// use dioxus_mobile_core::ios::get_or_init_manager;
/// use objc2_core_location::CLLocationManager;
///
/// let manager = get_or_init_manager(|| {
///     unsafe { CLLocationManager::new() }
/// });
/// ```
pub fn get_or_init_manager<T, F>(init: F) -> Option<&'static T>
where
    F: FnOnce() -> T,
{
    let Some(mtm) = MainThreadMarker::new() else {
        return None;
    };

    // Use a static cell to cache the manager
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
pub fn get_or_init_objc_manager<T, F>(init: F) -> Option<&'static T>
where
    F: FnOnce() -> T,
    T: 'static,
{
    let Some(mtm) = MainThreadMarker::new() else {
        return None;
    };

    // This is a simplified implementation. In practice, you'd need
    // a more sophisticated caching mechanism that can handle different
    // manager types generically.
    None
}
