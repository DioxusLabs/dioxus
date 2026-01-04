# Dioxus Manifest & Permissions System

Declare permissions, deep links, background modes, and platform-specific manifest settings through `Dioxus.toml` configuration.

## Overview

Dioxus provides a unified configuration system for cross-platform app manifest customization. Instead of using macros or platform-specific files, all manifest configuration is centralized in your `Dioxus.toml` file.

The system follows a **unified-first, platform-override** pattern:
1. Define cross-platform settings in unified sections (`[permissions]`, `[deep_links]`, `[background]`)
2. Override or extend with platform-specific sections (`[ios]`, `[android]`, `[macos]`)

## Permissions

Add a `[permissions]` section to your `Dioxus.toml`:

```toml
[bundle]
identifier = "com.example.myapp"

[permissions]
location = { precision = "fine", description = "Track your location for navigation" }
camera = { description = "Take photos for your profile" }
microphone = { description = "Record voice messages" }
```

The CLI automatically maps these unified permissions to platform-specific identifiers:

| Unified Permission | Android | iOS/macOS |
|-------------------|---------|-----------|
| `location.fine` | `ACCESS_FINE_LOCATION` | `NSLocationWhenInUseUsageDescription` |
| `location.coarse` | `ACCESS_COARSE_LOCATION` | `NSLocationWhenInUseUsageDescription` |
| `background_location` | `ACCESS_BACKGROUND_LOCATION` | `NSLocationAlwaysAndWhenInUseUsageDescription` |
| `camera` | `CAMERA` | `NSCameraUsageDescription` |
| `microphone` | `RECORD_AUDIO` | `NSMicrophoneUsageDescription` |
| `notifications` | `POST_NOTIFICATIONS` | (runtime only) |
| `photos.read` | `READ_MEDIA_IMAGES` | `NSPhotoLibraryUsageDescription` |
| `photos.write` | `WRITE_EXTERNAL_STORAGE` | `NSPhotoLibraryAddUsageDescription` |
| `bluetooth` | `BLUETOOTH_CONNECT` | `NSBluetoothAlwaysUsageDescription` |
| `contacts` | `READ_CONTACTS` | `NSContactsUsageDescription` |
| `calendar` | `READ_CALENDAR` | `NSCalendarsUsageDescription` |
| `biometrics` | `USE_BIOMETRIC` | `NSFaceIDUsageDescription` |
| `nfc` | `NFC` | `NFCReaderUsageDescription` |
| `motion` | `ACTIVITY_RECOGNITION` | `NSMotionUsageDescription` |
| `health` | `BODY_SENSORS` | `NSHealthShareUsageDescription` |
| `speech` | `RECORD_AUDIO` | `NSSpeechRecognitionUsageDescription` |

## Deep Linking

Configure URL schemes and universal/app links with the unified `[deep_links]` section:

```toml
[deep_links]
# Custom URL schemes (e.g., myapp://path)
schemes = ["myapp", "com.example.myapp"]

# Universal links / App Links hosts
hosts = ["example.com", "*.example.com"]

# Path patterns (optional, matches all paths if empty)
paths = ["/app/*", "/share/*"]
```

Platform-specific overrides extend (not replace) the unified config:

```toml
# iOS-only additional schemes
[ios]
url_schemes = ["myapp-ios"]

# Android-specific intent filters for advanced cases
[[android.intent_filters]]
actions = ["android.intent.action.VIEW"]
categories = ["android.intent.category.DEFAULT", "android.intent.category.BROWSABLE"]
auto_verify = true  # For App Links (requires assetlinks.json)

[[android.intent_filters.data]]
scheme = "https"
host = "example.com"
path_prefix = "/app"
```

## Background Modes

Configure background execution capabilities with the unified `[background]` section:

```toml
[background]
location = true           # Background location updates
audio = true              # Background audio playback
fetch = true              # Background data fetch
remote-notifications = true  # Remote push notification processing
voip = true               # VoIP calls
bluetooth = true          # Bluetooth LE accessories
processing = true         # Background processing tasks
```

Platform-specific overrides:

```toml
# iOS: Additional background modes
[ios]
background_modes = ["newsstand-content", "external-accessory"]

# Android: Foreground service types
[android]
foreground_service_types = ["location", "mediaPlayback", "phoneCall"]
```

## Platform-Specific Configuration

### iOS Configuration

```toml
[ios]
deployment_target = "15.0"
url_schemes = ["myapp-ios"]  # Platform-specific URL schemes
background_modes = ["location", "fetch"]  # Additional background modes

# Document types the app can open
[[ios.document_types]]
name = "My Document"
extensions = ["mydoc", "mydocx"]
role = "Editor"

# Add Info.plist entries as key-value pairs
[ios.plist]
ITSAppUsesNonExemptEncryption = false

# Add entitlements
[ios.entitlements]
"com.apple.security.application-groups" = ["group.com.example.app"]
"aps-environment" = "development"

# Raw XML for advanced cases
[ios.raw]
info_plist = """
<key>CustomKey</key>
<string>custom-value</string>
"""
```

### Android Configuration

```toml
[android]
min_sdk = 24
target_sdk = 34
features = ["android.hardware.location.gps"]
url_schemes = ["myapp-android"]  # Platform-specific URL schemes
foreground_service_types = ["location", "mediaPlayback"]

# Intent filters for deep linking
[[android.intent_filters]]
actions = ["android.intent.action.VIEW"]
categories = ["android.intent.category.DEFAULT", "android.intent.category.BROWSABLE"]
auto_verify = true

[[android.intent_filters.data]]
scheme = "https"
host = "example.com"

# Package visibility queries (Android 11+)
[android.queries]
packages = ["com.other.app"]

# Additional permissions not covered by unified permissions
[android.permissions]
"android.permission.FOREGROUND_SERVICE" = { description = "Run background service" }

# Raw manifest XML for advanced cases
[android.raw]
manifest = """
<uses-feature android:name="android.hardware.touchscreen" android:required="false" />
"""
```

### macOS Configuration

```toml
[macos]
minimum_system_version = "11.0"
frameworks = ["CoreLocation.framework"]
url_schemes = ["myapp-macos"]  # Platform-specific URL schemes
category = "public.app-category.productivity"

# Document types (same format as iOS)
[[macos.document_types]]
name = "My Format"
extensions = ["myfmt"]

# Add Info.plist entries
[macos.plist]
LSUIElement = true

# Add entitlements
[macos.entitlements]
"com.apple.security.app-sandbox" = true
"com.apple.security.network.client" = true

# Raw XML for advanced cases
[macos.raw]
info_plist = """
<key>CustomKey</key>
<string>custom-value</string>
"""
```

## Complete Example

Here's a complete example for a geolocation app:

```toml
[application]
name = "GeoTracker"

[bundle]
identifier = "com.example.geotracker"

# Unified permissions - automatically mapped to each platform
[permissions]
location = { precision = "fine", description = "Track your precise location for navigation" }
notifications = { description = "Send alerts when you arrive at destinations" }

# iOS-specific settings
[ios]
deployment_target = "15.0"

[ios.plist]
UIBackgroundModes = ["location"]

[ios.entitlements]
"com.apple.developer.healthkit" = false

# Android-specific settings
[android]
min_sdk = 24
target_sdk = 34
features = ["android.hardware.location.gps"]

# macOS-specific settings
[macos]
minimum_system_version = "11.0"

[macos.entitlements]
"com.apple.security.app-sandbox" = true
```

## How it Works

1. **Parse**: The CLI parses `Dioxus.toml` and extracts all permission and platform-specific configuration
2. **Map**: The `PermissionMapper` converts unified permissions to platform-specific identifiers
3. **Generate**: Handlebars templates inject the permissions and configuration into platform manifests:
   - `AndroidManifest.xml` for Android
   - `Info.plist` for iOS and macOS
4. **Bundle**: The final app bundle includes the configured permissions

## Migration from Macro-Based System

If you were previously using the `permission!()` macro, migrate to `Dioxus.toml`:

**Before (deprecated):**
```rust
use manganis::permission;

const LOCATION: Permission = permission!(
    PermissionBuilder::location(LocationPrecision::Fine)
        .with_description("Track your runs")
        .build()
);
```

**After:**
```toml
# Dioxus.toml
[permissions]
location = { precision = "fine", description = "Track your runs" }
```

The `permission!()` macro has been removed. All permissions should now be declared in `Dioxus.toml`.

## FFI Integration

For native plugins that require specific permissions, declare them in your library's documentation and let the app developer add them to their `Dioxus.toml`. The `#[manganis::ffi]` macro for FFI bindings is still available for Swift/Kotlin integration.
