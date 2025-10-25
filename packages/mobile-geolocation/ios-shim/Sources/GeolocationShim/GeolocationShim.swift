import CoreLocation
import Foundation

/// Swift shim for geolocation functionality on iOS.
///
/// This module provides C-compatible functions that can be called from Rust
/// via FFI to access CoreLocation APIs.

/// Get the last known location from CoreLocation.
///
/// Returns a pointer to a 2-element array [latitude, longitude],
/// or NULL if no location is available.
///
/// The caller must free the returned pointer using `free()`.
@_cdecl("ios_geoloc_last_known")
public func ios_geoloc_last_known() -> UnsafeMutablePointer<Double>? {
    let manager = CLLocationManager()
    
    // Get the last known location
    guard let location = manager.location else {
        return nil
    }
    
    // Allocate memory for the result
    let ptr = UnsafeMutablePointer<Double>.allocate(capacity: 2)
    ptr[0] = location.coordinate.latitude
    ptr[1] = location.coordinate.longitude
    
    return ptr
}

/// Request location authorization from the user.
///
/// This function requests "when in use" authorization, which allows
/// location access while the app is in the foreground.
///
/// For background location, you would need to call
/// `requestAlwaysAuthorization()` instead.
@_cdecl("ios_geoloc_request_authorization")
public func ios_geoloc_request_authorization() {
    let manager = CLLocationManager()
    manager.requestWhenInUseAuthorization()
}

/// Check if location services are enabled on the device.
///
/// Returns 1 if enabled, 0 if disabled.
@_cdecl("ios_geoloc_services_enabled")
public func ios_geoloc_services_enabled() -> Int32 {
    return CLLocationManager.locationServicesEnabled() ? 1 : 0
}

/// Get the current authorization status.
///
/// Returns:
/// - 0: Not determined
/// - 1: Restricted
/// - 2: Denied
/// - 3: Authorized (always)
/// - 4: Authorized (when in use)
@_cdecl("ios_geoloc_authorization_status")
public func ios_geoloc_authorization_status() -> Int32 {
    let status = CLLocationManager.authorizationStatus()
    switch status {
    case .notDetermined:
        return 0
    case .restricted:
        return 1
    case .denied:
        return 2
    case .authorizedAlways:
        return 3
    case .authorizedWhenInUse:
        return 4
    @unknown default:
        return 0
    }
}

