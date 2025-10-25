package com.dioxus.geoloc

import android.Manifest
import android.app.Activity
import android.content.pm.PackageManager
import android.location.Location
import android.location.LocationManager
import androidx.annotation.Keep
import androidx.core.app.ActivityCompat

/**
 * Kotlin shim for geolocation functionality.
 *
 * This object provides JNI-friendly static methods for accessing
 * Android's LocationManager from Rust code.
 */
@Keep
object GeolocationShim {
    /**
     * Get the last known location from the device.
     *
     * @param activity The current Android Activity
     * @return A DoubleArray [latitude, longitude] if available, null otherwise
     */
    @JvmStatic
    fun lastKnown(activity: Activity): DoubleArray? {
        // Check if we have location permissions
        val hasFinePermission = ActivityCompat.checkSelfPermission(
            activity,
            Manifest.permission.ACCESS_FINE_LOCATION
        ) == PackageManager.PERMISSION_GRANTED

        val hasCoarsePermission = ActivityCompat.checkSelfPermission(
            activity,
            Manifest.permission.ACCESS_COARSE_LOCATION
        ) == PackageManager.PERMISSION_GRANTED

        if (!hasFinePermission && !hasCoarsePermission) {
            // No permissions granted
            return null
        }

        // Get LocationManager
        val locationManager = activity.getSystemService(LocationManager::class.java)
            ?: return null

        // Try GPS provider first (most accurate)
        var location: Location? = null
        
        if (hasFinePermission) {
            try {
                location = locationManager.getLastKnownLocation(LocationManager.GPS_PROVIDER)
            } catch (e: SecurityException) {
                // Permission was revoked
            }
        }

        // Fall back to network provider if GPS unavailable
        if (location == null && hasCoarsePermission) {
            try {
                location = locationManager.getLastKnownLocation(LocationManager.NETWORK_PROVIDER)
            } catch (e: SecurityException) {
                // Permission was revoked
            }
        }

        // Return lat/lon as double array
        return location?.let { loc ->
            doubleArrayOf(loc.latitude, loc.longitude)
        }
    }

    /**
     * Request location permissions at runtime.
     *
     * This is a helper method for requesting permissions. The Rust code
     * should call this before attempting to get location.
     *
     * @param activity The current Android Activity
     * @param requestCode Request code for the permission callback
     * @param fine Whether to request fine (GPS) or coarse (network) location
     */
    @JvmStatic
    fun requestPermission(activity: Activity, requestCode: Int, fine: Boolean) {
        val permission = if (fine) {
            Manifest.permission.ACCESS_FINE_LOCATION
        } else {
            Manifest.permission.ACCESS_COARSE_LOCATION
        }

        ActivityCompat.requestPermissions(
            activity,
            arrayOf(permission),
            requestCode
        )
    }
}

