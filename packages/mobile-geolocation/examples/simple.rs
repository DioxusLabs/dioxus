//! Simple example demonstrating geolocation usage
//!
//! This example shows how to use the mobile-geolocation crate
//! to get the last known location on Android and iOS.
//!
//! Run with:
//! ```
//! cargo run --example simple --target aarch64-linux-android
//! cargo run --example simple --target aarch64-apple-ios
//! ```

use dioxus_mobile_geolocation::last_known_location;

fn main() {
    println!("Mobile Geolocation Example");
    println!("===========================\n");

    // Check which platform we're on
    #[cfg(target_os = "android")]
    println!("Platform: Android");

    #[cfg(target_os = "ios")]
    println!("Platform: iOS");

    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    {
        println!("Platform: Other (geolocation not supported)");
        println!("\nThis example only works on Android and iOS targets.");
        println!("Try building with:");
        println!("  cargo build --target aarch64-linux-android");
        println!("  cargo build --target aarch64-apple-ios");
        return;
    }

    // Attempt to get location
    println!("\nAttempting to get last known location...");

    match last_known_location() {
        Some((lat, lon)) => {
            println!("✅ Location found!");
            println!("   Latitude:  {:.6}°", lat);
            println!("   Longitude: {:.6}°", lon);
            println!("\n📍 View on map: https://www.google.com/maps?q={},{}", lat, lon);
        }
        None => {
            println!("❌ No location available");
            println!("\nPossible reasons:");
            println!("  • Location permissions not granted");
            println!("  • Location services disabled");
            println!("  • No cached location available");
            println!("\nMake sure to:");
            #[cfg(target_os = "android")]
            println!("  • Grant location permissions when prompted");
            #[cfg(target_os = "ios")]
            println!("  • Call CLLocationManager.requestWhenInUseAuthorization()");
            println!("  • Enable location services in device settings");
        }
    }

    println!("\n✨ Example complete!");
}

