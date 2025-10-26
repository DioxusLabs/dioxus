use objc2::rc::Retained;
use objc2::MainThreadMarker;
use objc2_core_location::{CLLocation, CLLocationManager, CLAuthorizationStatus};
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

    // Check authorization status first
    let auth_status = unsafe { manager.authorizationStatus() };
    
    // Only request if not determined (NotDetermined)
    match auth_status {
        CLAuthorizationStatus::NotDetermined => {
            unsafe {
                manager.requestWhenInUseAuthorization();
            }
        }
        _ => {} // Already determined, don't request again
    }

    true
}

/// Get the last known location
pub fn last_known() -> Option<(f64, f64)> {
    let Some(mtm) = MainThreadMarker::new() else {
        return None;
    };

    let manager = get_location_manager(mtm);

    // Check authorization status before attempting to get location
    let auth_status = unsafe { manager.authorizationStatus() };
    
    // Only proceed if authorized
    match auth_status {
        CLAuthorizationStatus::AuthorizedAlways | 
        CLAuthorizationStatus::AuthorizedWhenInUse => {
            // Can proceed to get location
        }
        _ => {
            // Not authorized - try to get last known location anyway
            // This might work for locations cached before permission was revoked
        }
    }

    // First, try to get the cached location without starting updates
    let location: Option<Retained<CLLocation>> = unsafe { manager.location() };
    
    if location.is_some() {
        let loc = location.unwrap();
        let coordinate = unsafe { loc.coordinate() };
        return Some((coordinate.latitude, coordinate.longitude));
    }

    // If no cached location, start updates
    // Note: In a proper implementation, we would set up a delegate to receive
    // location updates asynchronously. For now, we'll use a simple approach
    // that starts updates and then checks after a delay.
    unsafe {
        manager.startUpdatingLocation();
    }
    
    // Wait for location to be obtained (allowing GPS to get a fix)
    std::thread::sleep(std::time::Duration::from_millis(1000));

    // Try again now that updates are running
    let location: Option<Retained<CLLocation>> = unsafe { manager.location() };

    // Stop updating to conserve battery
    unsafe {
        manager.stopUpdatingLocation();
    }

    location.map(|loc| {
        let coordinate = unsafe { loc.coordinate() };
        (coordinate.latitude, coordinate.longitude)
    })
}

