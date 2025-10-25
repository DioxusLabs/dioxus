#ifndef GEOLOCATION_SHIM_H
#define GEOLOCATION_SHIM_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/// Get the last known location from CoreLocation.
///
/// Returns a pointer to a 2-element array [latitude, longitude],
/// or NULL if no location is available.
///
/// The caller must free the returned pointer.
double* ios_geoloc_last_known(void);

/// Request location authorization from the user.
void ios_geoloc_request_authorization(void);

/// Check if location services are enabled.
///
/// Returns 1 if enabled, 0 if disabled.
int32_t ios_geoloc_services_enabled(void);

/// Get the current authorization status.
///
/// Returns:
/// - 0: Not determined
/// - 1: Restricted
/// - 2: Denied
/// - 3: Authorized (always)
/// - 4: Authorized (when in use)
int32_t ios_geoloc_authorization_status(void);

#ifdef __cplusplus
}
#endif

#endif /* GEOLOCATION_SHIM_H */

