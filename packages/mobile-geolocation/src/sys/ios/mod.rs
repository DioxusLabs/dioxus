use objc2::rc::Retained;
use objc2::MainThreadMarker;
use objc2_core_location::{CLLocation, CLLocationManager};
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
            slot.as_ref().expect("LOCATION_MANAGER initialized")
        }
    }
}

// SAFETY: `MainThreadCell` enforces main-thread-only access through
// `MainThreadMarker`.
unsafe impl<T> Sync for MainThreadCell<T> {}

/// Global location manager instance
static LOCATION_MANAGER: MainThreadCell<Retained<CLLocationManager>> = MainThreadCell::new();

/// Get or create the global location manager
fn get_location_manager(mtm: MainThreadMarker) -> &'static Retained<CLLocationManager> {
    LOCATION_MANAGER.get_or_init_with(mtm, || {
        // SAFETY: `CLLocationManager` is main-thread-only; the marker provided to
        // `get_or_init_with` ensures we're on the main thread.
        unsafe { CLLocationManager::new() }
    })
}

/// Request location authorization
pub fn request_permission() -> bool {
    let Some(mtm) = MainThreadMarker::new() else {
        return false;
    };

    let manager = get_location_manager(mtm);

    // Request "when in use" authorization
    unsafe {
        manager.requestWhenInUseAuthorization();
    }

    true
}

/// Get the last known location
pub fn last_known() -> Option<(f64, f64)> {
    let Some(mtm) = MainThreadMarker::new() else {
        return None;
    };

    let manager = get_location_manager(mtm);

    // Get the current location
    let location: Option<Retained<CLLocation>> = unsafe { manager.location() };

    location.map(|loc| {
        let coordinate = unsafe { loc.coordinate() };
        (coordinate.latitude, coordinate.longitude)
    })
}
