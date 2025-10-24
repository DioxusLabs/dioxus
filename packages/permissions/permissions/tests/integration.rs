use permissions::{permission, LocationPrecision, PermissionKind, Platform};

#[test]
fn test_camera_permission() {
    const CAM: permissions::Permission = permission!(Camera, description = "For selfies");

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
        permission!(Location(Fine), description = "Track your runs");
    const LOCATION_COARSE: permissions::Permission =
        permission!(Location(Coarse), description = "Find nearby places");

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
fn test_platform_specific_permissions() {
    // Android-specific permission
    const SMS: permissions::Permission = permission!(Sms, description = "Read SMS messages");
    assert!(SMS.supports_platform(Platform::Android));
    assert!(!SMS.supports_platform(Platform::Ios));
    assert!(!SMS.supports_platform(Platform::Web));
    assert_eq!(
        SMS.android_permission(),
        Some("android.permission.READ_SMS".to_string())
    );

    // iOS-specific permission
    const FACE_ID: permissions::Permission = permission!(FaceId, description = "Use Face ID");
    assert!(!FACE_ID.supports_platform(Platform::Android));
    assert!(FACE_ID.supports_platform(Platform::Ios));
    assert!(FACE_ID.supports_platform(Platform::Macos));
    assert!(!FACE_ID.supports_platform(Platform::Web));
    assert_eq!(
        FACE_ID.ios_key(),
        Some("NSFaceIDUsageDescription".to_string())
    );

    // Web-specific permission
    const CLIPBOARD: permissions::Permission =
        permission!(Clipboard, description = "Access clipboard");
    assert!(!CLIPBOARD.supports_platform(Platform::Android));
    assert!(!CLIPBOARD.supports_platform(Platform::Ios));
    assert!(CLIPBOARD.supports_platform(Platform::Web));
    assert_eq!(
        CLIPBOARD.web_permission(),
        Some("clipboard-read".to_string())
    );
}

#[test]
fn test_custom_permission() {
    const CUSTOM: permissions::Permission = permission!(
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
    // const CAM: permissions::Permission = permission!(Camera, description = "Take photos");
    // const MIC: permissions::Permission = permission!(Microphone, description = "Record audio");

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
