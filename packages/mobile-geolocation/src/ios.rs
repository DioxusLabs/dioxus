//! iOS geolocation implementation via Objective-C FFI

#[link(name = "GeolocationShim", kind = "static")]
extern "C" {
    /// Initialize the location manager
    fn ios_geoloc_init();
    
    /// Get the last known location from iOS CoreLocation.
    ///
    /// Returns a pointer to a 2-element array [latitude, longitude],
    /// or null if no location is available.
    ///
    /// The caller is responsible for freeing the returned pointer.
    fn ios_geoloc_last_known() -> *mut f64;
    
    /// Request location authorization from the user
    fn ios_geoloc_request_authorization();
    
    /// Check if location services are enabled
    fn ios_geoloc_services_enabled() -> i32;
    
    /// Get the current authorization status
    fn ios_geoloc_authorization_status() -> i32;
}

/// Request location permissions
pub fn request_permission() -> bool {
    unsafe {
        ios_geoloc_init();
        ios_geoloc_request_authorization();
        true // iOS permission requests are always "sent" (user sees dialog)
    }
}

/// Get the last known location from iOS's CLLocationManager.
///
/// This function calls into the Objective-C shim which queries CoreLocation
/// for the last cached location.
///
/// Returns `Some((latitude, longitude))` if available, `None` otherwise.
pub fn last_known() -> Option<(f64, f64)> {
    unsafe {
        ios_geoloc_init();
        
        // Check if location services are enabled
        if ios_geoloc_services_enabled() == 0 {
            eprintln!("Location services are disabled on this device");
            return None;
        }
        
        // Check authorization status
        let status = ios_geoloc_authorization_status();
        if status == 0 { // Not determined
            eprintln!("Location permission not determined - requesting permission");
            ios_geoloc_request_authorization();
            return None;
        } else if status == 1 || status == 2 { // Restricted or denied
            eprintln!("Location permission denied or restricted (status: {})", status);
            return None;
        }
        
        let ptr = ios_geoloc_last_known();
        if ptr.is_null() {
            eprintln!("No location available from CoreLocation");
            return None;
        }

        let lat = *ptr.add(0);
        let lon = *ptr.add(1);

        // Free the allocated memory
        libc::free(ptr as *mut libc::c_void);

        eprintln!("Successfully retrieved iOS location: lat={}, lon={}", lat, lon);
        Some((lat, lon))
    }
}

