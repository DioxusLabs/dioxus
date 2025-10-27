use permissions::{static_permission, LocationPrecision, PermissionKind, Platform};

#[test]
fn test_camera_permission() {
    const CAM: permissions::Permission = static_permission!(Camera, description = "For selfies");

    assert_eq!(CAM.description(), "For selfies");
    assert!(CAM.supports_platform(Platform::Android));
    assert!(CAM.supports_platform(Platform::Ios));
    assert!(CAM.supports_platform(Platform::Macos));
    assert!(CAM.supports_platform(Platform::Windows));
    assert!(CAM.supports_platform(Platform::Web));
    assert!(!CAM.supports_platform(Platform::Linux));

    assert_eq!(
        CAM.android_permission(),
        Some("android.permission.CAMERA".to_string())
    );
    assert_eq!(CAM.ios_key(), Some("NSCameraUsageDescription".to_string()));
    assert_eq!(
        CAM.macos_key(),
        Some("NSCameraUsageDescription".to_string())
    );
    assert_eq!(CAM.windows_capability(), Some("webcam".to_string()));
    assert_eq!(CAM.web_permission(), Some("camera".to_string()));
}

#[test]
fn test_location_permission() {
    const LOCATION_FINE: permissions::Permission =
        static_permission!(Location(Fine), description = "Track your runs");
    const LOCATION_COARSE: permissions::Permission =
        static_permission!(Location(Coarse), description = "Find nearby places");

    assert_eq!(LOCATION_FINE.description(), "Track your runs");
    assert_eq!(LOCATION_COARSE.description(), "Find nearby places");

    assert_eq!(
        LOCATION_FINE.android_permission(),
        Some("android.permission.ACCESS_FINE_LOCATION".to_string())
    );
    assert_eq!(
        LOCATION_COARSE.android_permission(),
        Some("android.permission.ACCESS_COARSE_LOCATION".to_string())
    );

    assert_eq!(
        LOCATION_FINE.ios_key(),
        Some("NSLocationAlwaysAndWhenInUseUsageDescription".to_string())
    );
    assert_eq!(
        LOCATION_COARSE.ios_key(),
        Some("NSLocationWhenInUseUsageDescription".to_string())
    );
}

#[test]
fn test_microphone_permission() {
    const MIC: permissions::Permission =
        static_permission!(Microphone, description = "Record audio");

    assert_eq!(MIC.description(), "Record audio");
    assert!(MIC.supports_platform(Platform::Android));
    assert!(MIC.supports_platform(Platform::Ios));
    assert!(MIC.supports_platform(Platform::Macos));
    assert!(MIC.supports_platform(Platform::Windows));
    assert!(MIC.supports_platform(Platform::Web));
    assert!(!MIC.supports_platform(Platform::Linux));

    assert_eq!(
        MIC.android_permission(),
        Some("android.permission.RECORD_AUDIO".to_string())
    );
    assert_eq!(MIC.ios_key(), Some("NSMicrophoneUsageDescription".to_string()));
    assert_eq!(
        MIC.macos_key(),
        Some("NSMicrophoneUsageDescription".to_string())
    );
    assert_eq!(MIC.windows_capability(), Some("microphone".to_string()));
    assert_eq!(MIC.web_permission(), Some("microphone".to_string()));
}

#[test]
fn test_notifications_permission() {
    const NOTIF: permissions::Permission =
        static_permission!(Notifications, description = "Send you notifications");

    assert_eq!(NOTIF.description(), "Send you notifications");
    assert!(NOTIF.supports_platform(Platform::Android));
    assert!(!NOTIF.supports_platform(Platform::Ios)); // Runtime only
    assert!(!NOTIF.supports_platform(Platform::Macos)); // Runtime only
    assert!(NOTIF.supports_platform(Platform::Web));

    assert_eq!(
        NOTIF.android_permission(),
        Some("android.permission.POST_NOTIFICATIONS".to_string())
    );
    assert_eq!(NOTIF.ios_key(), None); // No build-time permission
    assert_eq!(NOTIF.web_permission(), Some("notifications".to_string()));
}

#[test]
fn test_custom_for_platform_specific_permissions() {
    // Example: Accessing contacts on Android/iOS/macOS using Custom
    // (This is not in the tested set, so we use Custom)
    const CONTACTS: permissions::Permission = static_permission!(
        Custom {
            android = "android.permission.READ_CONTACTS",
            ios = "NSContactsUsageDescription",
            macos = "NSContactsUsageDescription",
            windows = "contacts",
            linux = "",
            web = "contacts"
        },
        description = "Access your contacts"
    );
    
    assert!(CONTACTS.supports_platform(Platform::Android));
    assert_eq!(
        CONTACTS.android_permission(),
        Some("android.permission.READ_CONTACTS".to_string())
    );
    assert_eq!(
        CONTACTS.ios_key(),
        Some("NSContactsUsageDescription".to_string())
    );
}

#[test]
fn test_custom_permission() {
    const CUSTOM: permissions::Permission = static_permission!(
        Custom {
            android = "MY_PERM",
            ios = "NSMyUsage",
            macos = "NSMyUsage",
            windows = "myCap",
            linux = "my_perm",
            web = "my-perm"
        },
        description = "Custom permission"
    );

    assert_eq!(CUSTOM.description(), "Custom permission");
    assert_eq!(CUSTOM.android_permission(), Some("MY_PERM".to_string()));
    assert_eq!(CUSTOM.ios_key(), Some("NSMyUsage".to_string()));
    assert_eq!(CUSTOM.macos_key(), Some("NSMyUsage".to_string()));
    assert_eq!(CUSTOM.windows_capability(), Some("myCap".to_string()));
    assert_eq!(CUSTOM.linux_permission(), Some("my_perm".to_string()));
    assert_eq!(CUSTOM.web_permission(), Some("my-perm".to_string()));
}

#[test]
fn test_permission_manifest() {
    use permissions::PermissionManifest;

    let manifest = PermissionManifest::new();
    assert!(manifest.is_empty());
    assert_eq!(manifest.len(), 0);

    // Note: In a real implementation, we would add permissions to the manifest
    // For now, we just test the basic structure
    // const CAM: permissions::Permission = static_permission!(Camera, description = "Take photos");
    // const MIC: permissions::Permission = static_permission!(Microphone, description = "Record audio");

    // Note: In a real implementation, we would add permissions to the manifest
    // For now, we just test the basic structure
    assert!(manifest.is_empty());
}

#[test]
fn test_permission_kind_mappings() {
    // Test that permission kinds map to correct platform identifiers
    let camera = PermissionKind::Camera;
    let identifiers = camera.platform_identifiers();

    assert_eq!(
        identifiers.android,
        Some(const_serialize::ConstStr::new("android.permission.CAMERA"))
    );
    assert_eq!(
        identifiers.ios,
        Some(const_serialize::ConstStr::new("NSCameraUsageDescription"))
    );
    assert_eq!(
        identifiers.web,
        Some(const_serialize::ConstStr::new("camera"))
    );

    let location_fine = PermissionKind::Location(LocationPrecision::Fine);
    let location_identifiers = location_fine.platform_identifiers();

    assert_eq!(
        location_identifiers.android,
        Some(const_serialize::ConstStr::new(
            "android.permission.ACCESS_FINE_LOCATION"
        ))
    );
    assert_eq!(
        location_identifiers.ios,
        Some(const_serialize::ConstStr::new(
            "NSLocationAlwaysAndWhenInUseUsageDescription"
        ))
    );
}

#[test]
fn test_platform_flags() {
    use permissions::PlatformFlags;

    let mobile = PlatformFlags::mobile();
    assert!(mobile.supports(Platform::Android));
    assert!(mobile.supports(Platform::Ios));
    assert!(!mobile.supports(Platform::Web));

    let desktop = PlatformFlags::desktop();
    assert!(!desktop.supports(Platform::Android));
    assert!(!desktop.supports(Platform::Ios));
    assert!(desktop.supports(Platform::Macos));
    assert!(desktop.supports(Platform::Windows));
    assert!(desktop.supports(Platform::Linux));

    let all = PlatformFlags::all();
    assert!(all.supports(Platform::Android));
    assert!(all.supports(Platform::Ios));
    assert!(all.supports(Platform::Macos));
    assert!(all.supports(Platform::Windows));
    assert!(all.supports(Platform::Linux));
    assert!(all.supports(Platform::Web));
}
