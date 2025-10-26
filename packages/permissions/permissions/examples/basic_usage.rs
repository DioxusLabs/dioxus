//! Basic usage example for the permissions crate
//!
//! This example demonstrates how to declare and use permissions across different platforms.

use permissions::{static_permission, Platform};

fn main() {
    // Declare various permissions
    const CAMERA: permissions::Permission = static_permission!(Camera, description = "Take photos");
    const LOCATION: permissions::Permission =
    static_permission!(Location(Fine), description = "Track your runs");
    const MICROPHONE: permissions::Permission =
    static_permission!(Microphone, description = "Record audio");
    const NOTIFICATIONS: permissions::Permission =
    static_permission!(Notifications, description = "Send push notifications");

    const CUSTOM: permissions::Permission = static_permission!(
        Custom {
            android = "MY_CUSTOM",
            ios = "NSMyCustom",
            macos = "NSMyCustom",
            windows = "myCustom",
            linux = "my_custom",
            web = "my-custom"
        },
        description = "Custom permission"
    );

    println!("=== Permission Information ===");

    // Display camera permission info
    println!("\n📷 Camera Permission:");
    println!("  Description: {}", CAMERA.description());
    println!("  Android: {:?}", CAMERA.android_permission());
    println!("  iOS: {:?}", CAMERA.ios_key());
    println!("  macOS: {:?}", CAMERA.macos_key());
    println!("  Windows: {:?}", CAMERA.windows_capability());
    println!("  Web: {:?}", CAMERA.web_permission());

    // Display location permission info
    println!("\n📍 Location Permission:");
    println!("  Description: {}", LOCATION.description());
    println!("  Android: {:?}", LOCATION.android_permission());
    println!("  iOS: {:?}", LOCATION.ios_key());
    println!("  Web: {:?}", LOCATION.web_permission());

    // Display microphone permission info
    println!("\n🎤 Microphone Permission:");
    println!("  Description: {}", MICROPHONE.description());
    println!("  Android: {:?}", MICROPHONE.android_permission());
    println!("  iOS: {:?}", MICROPHONE.ios_key());
    println!("  Web: {:?}", MICROPHONE.web_permission());

    // Display notifications permission info
    println!("\n🔔 Notifications Permission:");
    println!("  Description: {}", NOTIFICATIONS.description());
    println!("  Android: {:?}", NOTIFICATIONS.android_permission());
    println!("  iOS: {:?}", NOTIFICATIONS.ios_key());
    println!("  Web: {:?}", NOTIFICATIONS.web_permission());

    // Display custom permission info
    println!("\n🔧 Custom Permission:");
    println!("  Description: {}", CUSTOM.description());
    println!("  Android: {:?}", CUSTOM.android_permission());
    println!("  iOS: {:?}", CUSTOM.ios_key());
    println!("  macOS: {:?}", CUSTOM.macos_key());
    println!("  Windows: {:?}", CUSTOM.windows_capability());
    println!("  Linux: {:?}", CUSTOM.linux_permission());
    println!("  Web: {:?}", CUSTOM.web_permission());

    // Check platform support
    println!("\n=== Platform Support ===");

    let platforms = [
        Platform::Android,
        Platform::Ios,
        Platform::Macos,
        Platform::Windows,
        Platform::Linux,
        Platform::Web,
    ];

    for platform in platforms {
        println!("\n{} Platform:", format!("{:?}", platform));
        println!("  Camera: {}", CAMERA.supports_platform(platform));
        println!("  Location: {}", LOCATION.supports_platform(platform));
        println!("  Microphone: {}", MICROPHONE.supports_platform(platform));
        println!(
            "  Notifications: {}",
            NOTIFICATIONS.supports_platform(platform)
        );
        println!("  Custom: {}", CUSTOM.supports_platform(platform));
    }

    // Demonstrate permission manifest
    println!("\n=== Permission Manifest ===");

    use permissions::PermissionManifest;
    let manifest = PermissionManifest::new();

    // In a real implementation, permissions would be added to the manifest
    // For this example, we just show the structure
    println!("Manifest is empty: {}", manifest.is_empty());
    println!("Manifest length: {}", manifest.len());

    // Show platform-specific permissions
    println!("\nAndroid permissions:");
    for platform in platforms {
        let permissions = manifest.permissions_for_platform(platform);
        println!(
            "  {}: {} permissions",
            format!("{:?}", platform),
            permissions.len()
        );
    }
}
