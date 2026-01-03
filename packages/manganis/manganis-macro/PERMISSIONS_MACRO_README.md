# Permissions Macro

Procedural macro for declaring permissions with linker embedding.

This crate provides the `permission!()` and `permission!()` macros that allow you to declare permissions
that will be embedded in the binary using linker sections, similar to how Manganis
embeds assets. Use `permission!()` when you want to make it explicit that a
permission is a compile-time (linker) declaration that should be emitted into
platform manifests (Info.plist, AndroidManifest.xml, etc.). The `permission!()`
alias is kept for backward compatibility.

## Usage

The macro accepts any expression that evaluates to a `Permission`. There are two patterns:

### Builder Pattern (for Location and Custom permissions)

Location and custom permissions use the builder pattern:

```rust
use permissions::{Permission, PermissionBuilder, LocationPrecision};
use permissions_macro::permission;

// Location permission with fine precision
const LOCATION_FINE: Permission = permission!(
    PermissionBuilder::location(LocationPrecision::Fine)
        .with_description("Track your runs")
        .build()
);

// Location permission with coarse precision
const LOCATION_COARSE: Permission = permission!(
    PermissionBuilder::location(LocationPrecision::Coarse)
        .with_description("Approximate location")
        .build()
);

// Custom permission
const CUSTOM: Permission = permission!(
    PermissionBuilder::custom()
        .with_android("android.permission.MY_PERMISSION")
        .with_ios("NSMyUsageDescription")
        .with_macos("NSMyUsageDescription")
        .with_description("Custom permission")
        .build()
);
```

### Direct Construction (for simple permissions)

Simple permissions like Camera, Microphone, and Notifications use direct construction:

```rust
use permissions::{Permission, PermissionKind};
use permissions_macro::permission;

// Camera permission
const CAMERA: Permission = permission!(
    Permission::new(PermissionKind::Camera, "Take photos")
);

// Microphone permission
const MICROPHONE: Permission = permission!(
    Permission::new(PermissionKind::Microphone, "Record audio")
);

// Notifications permission
const NOTIFICATIONS: Permission = permission!(
    Permission::new(PermissionKind::Notifications, "Send notifications")
);
```

## How it works

The macro generates code that:

1. Creates a `Permission` instance with the specified kind and description
2. Serializes the permission data into a const buffer
3. Embeds the data in a linker section with a unique symbol name (`__PERMISSION__<hash>`)
4. Returns a `Permission` that can read the embedded data at runtime

This allows build tools to extract all permission declarations from the binary
by scanning for `__PERMISSION__*` symbols.
