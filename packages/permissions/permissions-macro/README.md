# Permissions Macro

Procedural macro for declaring permissions with linker embedding.

This crate provides the `permission!()` and `static_permission!()` macros that allow you to declare permissions
that will be embedded in the binary using linker sections, similar to how Manganis
embeds assets. Use `static_permission!()` when you want to make it explicit that a
permission is a compile-time (linker) declaration that should be emitted into
platform manifests (Info.plist, AndroidManifest.xml, etc.). The `permission!()`
alias is kept for backward compatibility.

## Usage

```rust
use permissions_core::Permission;
use permissions_macro::static_permission;

// Basic permission
const CAMERA: Permission = static_permission!(Camera, description = "Take photos");

// Location with precision
const LOCATION: Permission = static_permission!(Location(Fine), description = "Track your runs");

// Custom permission (not shown in doctests due to buffer size limitations)
// const CUSTOM: Permission = permission!(
//     Custom { 
//         android = "android.permission.MY_PERMISSION",
//         ios = "NSMyUsageDescription",
//         macos = "NSMyUsageDescription", 
//         windows = "myCapability",
//         linux = "my_permission",
//         web = "my-permission"
//     },
//     description = "Custom permission"
// );
```

## How it works

The macro generates code that:

1. Creates a `Permission` instance with the specified kind and description
2. Serializes the permission data into a const buffer
3. Embeds the data in a linker section with a unique symbol name (`__PERMISSION__<hash>`)
4. Returns a `Permission` that can read the embedded data at runtime

This allows build tools to extract all permission declarations from the binary
by scanning for `__PERMISSION__*` symbols.
