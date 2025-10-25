//! iOS geolocation implementation via Swift FFI

#[link(name = "GeolocationShim", kind = "static")]
extern "C" {
    /// Get the last known location from iOS CoreLocation.
    ///
    /// Returns a pointer to a 2-element array [latitude, longitude],
    /// or null if no location is available.
    ///
    /// The caller is responsible for freeing the returned pointer.
    fn ios_geoloc_last_known() -> *mut f64;
}

/// Get the last known location from iOS's CLLocationManager.
///
/// This function calls into the Swift shim which queries CoreLocation
/// for the last cached location.
///
/// Returns `Some((latitude, longitude))` if available, `None` otherwise.
pub fn last_known() -> Option<(f64, f64)> {
    unsafe {
        let ptr = ios_geoloc_last_known();
        if ptr.is_null() {
            return None;
        }

        let lat = *ptr.add(0);
        let lon = *ptr.add(1);

        // Free the Swift-allocated memory
        // Note: In production, you might want to expose a separate free function
        // from Swift to ensure proper deallocation
        libc::free(ptr as *mut libc::c_void);

        Some((lat, lon))
    }
}

