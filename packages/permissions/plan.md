# Permission Manager System with Linker-Based Collection

## Overview

Build a standalone permission management system inspired by Manganis that uses linker sections to collect permissions declared throughout the codebase and embed them into the binary. The system focuses on core functionality without CLI integration, making it ready for future build tool integration.

## Architecture

Three interconnected packages mirroring Manganis structure:

1. **permissions-core** - Core types, platform mappings, serialization
2. **permissions-macro** - Procedural macro with linker section generation
3. **permissions** - Public API crate

## Cross-Platform Permission Architecture

### Platform Categories

1. **Mobile**: Android, iOS
2. **Desktop**: macOS, Windows, Linux
3. **Web**: Browser APIs

### Permission Mapping Strategy

Each permission kind maps to platform-specific requirements:

**Camera Permission**:

- Android: `android.permission.CAMERA`
- iOS: `NSCameraUsageDescription` (Info.plist)
- macOS: `NSCameraUsageDescription` (Info.plist + entitlements)
- Windows: App capability declaration (Package.appxmanifest)
- Linux: No system-level permission (direct access)
- Web: Browser `getUserMedia()` API (runtime prompt)

**Location Permission**:

- Android: `ACCESS_FINE_LOCATION` / `ACCESS_COARSE_LOCATION`
- iOS: `NSLocationWhenInUseUsageDescription` / `NSLocationAlwaysUsageDescription`
- macOS: `NSLocationUsageDescription`
- Windows: Location capability
- Linux: No system-level permission
- Web: Geolocation API (runtime prompt)

**Microphone Permission**:

- Android: `RECORD_AUDIO`
- iOS: `NSMicrophoneUsageDescription`
- macOS: `NSMicrophoneUsageDescription`
- Windows: Microphone capability
- Linux: No system-level permission (PulseAudio/ALSA access)
- Web: `getUserMedia()` API

**Notification Permission**:

- Android: Runtime permission (API 33+)
- iOS: Runtime request via `UNUserNotificationCenter`
- macOS: Runtime request
- Windows: No permission required
- Linux: No permission required
- Web: Notification API (runtime prompt)

**File System Access**:

- Android: `READ_EXTERNAL_STORAGE` / `WRITE_EXTERNAL_STORAGE`
- iOS: Photo Library requires `NSPhotoLibraryUsageDescription`
- macOS: Sandbox entitlements
- Windows: BroadFileSystemAccess capability
- Linux: No system-level permission
- Web: File System Access API (runtime prompt)

**Network/Internet**:

- Android: `INTERNET`, `ACCESS_NETWORK_STATE`
- iOS: No explicit permission
- macOS: Outgoing connections allowed, incoming needs entitlements
- Windows: Internet capability
- Linux: No permission required
- Web: No permission required (CORS restrictions apply)

**Bluetooth**:

- Android: `BLUETOOTH`, `BLUETOOTH_ADMIN`, `BLUETOOTH_CONNECT` (API 31+)
- iOS: `NSBluetoothAlwaysUsageDescription`
- macOS: `NSBluetoothAlwaysUsageDescription`
- Windows: Bluetooth capability
- Linux: No system-level permission
- Web: Web Bluetooth API (runtime prompt)

### Platform-Specific Permissions

**Android-only**:

- `SYSTEM_ALERT_WINDOW`, `READ_SMS`, `READ_PHONE_STATE`, `CALL_PHONE`

**iOS/macOS-only**:

- `NSUserTrackingUsageDescription`, `NSFaceIDUsageDescription`, `NSLocalNetworkUsageDescription`

**Windows-only**:

- `appointments`, `contacts`, `enterpriseAuthentication`, `phoneCall`

**Web-only**:

- `clipboard-read`, `clipboard-write`, `payment-handler`, `screen-wake-lock`

## Key Components

### 1. Core Types (`packages/permissions/permissions-core/src`)

**`lib.rs`**: Module exports

**`permission.rs`**: Core permission structure

```rust
pub struct Permission {
    kind: PermissionKind,
    description: ConstStr,
    android_permissions: ConstVec<ConstStr>,  // Multiple Android permissions if needed
    ios_key: ConstStr,
    platforms: PlatformFlags,
}
```

**`platforms.rs`**: Platform definitions and mappings

```rust
pub enum PermissionKind {
    // Cross-platform
    Camera,
    Location(LocationPrecision),
    Microphone,
    PhotoLibrary,
    Contacts,
    // Android-specific
    Internet,
    NetworkState,
    // iOS-specific
    FaceId,
    UserTracking,
    // Custom (for future extensibility)
    Custom { android: &'static str, ios: &'static str },
}

pub enum LocationPrecision {
    Fine,      // Android: FINE_LOCATION, iOS: AlwaysAndWhenInUse
    Coarse,    // Android: COARSE_LOCATION, iOS: WhenInUse
}
```

Implement `SerializeConst` and `PartialEq`/`Hash` for all types using `const-serialize-macro`.

### 2. Macro Implementation (`packages/permissions/permissions-macro/src`)

**`lib.rs`**: Main macro entry point

```rust
#[proc_macro]
pub fn permission(input: TokenStream) -> TokenStream
```

**`permission.rs`**: Parse permission declarations

- Parse syntax: `permission!(Camera, description = "Take photos")`
- Support location precision: `permission!(Location(Fine), description = "Track your runs")`
- Support custom permissions: `permission!(Custom { android = "MY_PERMISSION", ios = "NSMyUsageDescription" }, description = "...")`
- Hash declaration for unique symbols

**`linker.rs`**: Generate linker sections (mirrors `manganis-macro/src/linker.rs`)

```rust
pub fn generate_link_section(permission: impl ToTokens, permission_hash: &str) -> TokenStream2
```

- Create `__PERMISSION__<hash>` export symbol
- Serialize permission to `ConstVec<u8>`
- Generate static array with `#[export_name]`
- Force reference to prevent optimization

### 3. Public API (`packages/permissions/src`)

**`lib.rs`**: Re-exports

```rust
pub use permissions_macro::permission;
pub use permissions_core::{Permission, PermissionKind, LocationPrecision, PlatformFlags};

#[doc(hidden)]
pub mod macro_helpers {
    pub use const_serialize::{self, ConstVec, ConstStr};
    pub use permissions_core::Permission;
    
    pub const fn serialize_permission(p: &Permission) -> ConstVec<u8> { ... }
    pub const fn copy_bytes<const N: usize>(bytes: &[u8]) -> [u8; N] { ... }
}
```

**`macro_helpers.rs`**: Helper functions for macro expansion

## Macro Expansion Example

### Input

```rust
const CAMERA: Permission = permission!(Camera, description = "Take photos of your food");
```

### Expanded Output

```rust
const CAMERA: Permission = {
    const __PERMISSION: Permission = Permission::new(
        PermissionKind::Camera,
        "Take photos of your food",
    );
    
    // Serialize to const buffer
    const __BUFFER: permissions::macro_helpers::ConstVec<u8> = 
        permissions::macro_helpers::serialize_permission(&__PERMISSION);
    const __BYTES: &[u8] = __BUFFER.as_ref();
    const __LEN: usize = __BYTES.len();
    
    // Embed in linker section with unique symbol
    #[export_name = "__PERMISSION__a1b2c3d4e5f6"]
    static __LINK_SECTION: [u8; __LEN] = permissions::macro_helpers::copy_bytes(__BYTES);
    
    // Force reference to prevent dead code elimination
    static __REFERENCE_TO_LINK_SECTION: &'static [u8] = &__LINK_SECTION;
    
    Permission::from_embedded(|| unsafe { 
        std::ptr::read_volatile(&__REFERENCE_TO_LINK_SECTION) 
    })
};
```

## Package Structure

```
packages/permissions/
├── permissions/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       └── macro_helpers.rs
├── permissions-core/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── permission.rs
│       └── platforms.rs
└── permissions-macro/
    ├── Cargo.toml
    └── src/
        ├── lib.rs
        ├── linker.rs
        └── permission.rs
```

## Cargo.toml Dependencies

**permissions-core/Cargo.toml**:

```toml
[dependencies]
const-serialize = { path = "../../const-serialize" }
const-serialize-macro = { path = "../../const-serialize-macro" }
serde = { version = "1.0", features = ["derive"] }
```

**permissions-macro/Cargo.toml**:

```toml
[dependencies]
syn = { version = "2.0", features = ["full"] }
quote = "1.0"
proc-macro2 = "1.0"
```

**permissions/Cargo.toml**:

```toml
[dependencies]
permissions-core = { path = "../permissions-core" }
permissions-macro = { path = "../permissions-macro" }
const-serialize = { path = "../../const-serialize" }
```

## Testing Strategy

### Unit Tests

- `permissions-macro`: Test macro parsing for various permission syntaxes
- `permissions-core`: Test serialization/deserialization round-trips
- Platform mapping correctness

### Integration Tests

- Create test binary with multiple permission declarations
- Verify symbols are embedded (check with `nm` or similar)
- Verify permissions can be extracted and deserialized

### Example Test

```rust
// tests/integration.rs in permissions crate
#[test]
fn test_camera_permission() {
    const CAM: Permission = permission!(Camera, description = "For selfies");
    assert_eq!(CAM.kind(), PermissionKind::Camera);
    assert_eq!(CAM.android_permissions(), &["android.permission.CAMERA"]);
    assert_eq!(CAM.ios_key(), "NSCameraUsageDescription");
}
```

## Future Integration Points (for reference, not implemented now)

The embedded `__PERMISSION__` symbols can later be extracted by:

1. CLI reading binary symbol table (like `packages/cli/src/build/assets.rs`)
2. Injecting into AndroidManifest.xml
3. Injecting into Info.plist
4. Generating permission request code

## Design Decisions

1. **Const-time everything**: All permission data computed at compile time
2. **Linker-based collection**: No runtime registration, no global state
3. **Platform-agnostic core**: Unified API, platform details in mappings
4. **Extensible**: Custom permission kind for uncommon permissions
5. **Type-safe**: Strongly typed permission kinds, not strings