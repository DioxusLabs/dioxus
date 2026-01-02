#![doc = include_str!("../README.md")]
#![deny(missing_docs)]

use proc_macro::TokenStream;
use quote::quote;
use syn::parse_macro_input;

pub(crate) mod linker;
pub(crate) mod permission;

use permission::PermissionParser;

/// Declare a permission that will be embedded in the binary
///
/// # Syntax
///
/// The macro accepts any expression that evaluates to a `Permission`. There are two patterns:
///
/// ## Builder Pattern (for Location and Custom permissions)
///
/// Location permissions use the builder pattern:
/// ```rust
/// use permissions::{Permission, PermissionBuilder, LocationPrecision};
/// use permissions_macro::static_permission;
///
/// // Fine location
/// const LOCATION_FINE: Permission = static_permission!(
///     PermissionBuilder::location(LocationPrecision::Fine)
///         .with_description("Track your runs")
///         .build()
/// );
///
/// // Coarse location
/// const LOCATION_COARSE: Permission = static_permission!(
///     PermissionBuilder::location(LocationPrecision::Coarse)
///         .with_description("Approximate location")
///         .build()
/// );
///
/// // Custom permission
/// const CUSTOM: Permission = static_permission!(
///     PermissionBuilder::custom()
///         .with_android("android.permission.MY_PERMISSION")
///         .with_ios("NSMyUsageDescription")
///         .with_macos("NSMyUsageDescription")
///         .with_description("Custom permission")
///         .build()
/// );
/// ```
///
/// ## Direct Construction (for simple permissions)
///
/// Simple permissions like Camera, Microphone, and Notifications use direct construction:
/// ```rust
/// use permissions::{Permission, PermissionKind};
/// use permissions_macro::static_permission;
///
/// const CAMERA: Permission = static_permission!(
///     Permission::new(PermissionKind::Camera, "Take photos")
/// );
///
/// const MICROPHONE: Permission = static_permission!(
///     Permission::new(PermissionKind::Microphone, "Record audio")
/// );
///
/// const NOTIFICATIONS: Permission = static_permission!(
///     Permission::new(PermissionKind::Notifications, "Send notifications")
/// );
/// ```
///
/// # Supported Permission Kinds
///
/// Only tested and verified permissions are included. For any other permissions,
/// use the `Custom` variant with platform-specific identifiers.
///
/// ## ✅ Tested Permissions (Only for requesting permissions)
///
/// - `Camera` - Camera access (tested across all platforms)
/// - `Location(Fine)` / `Location(Coarse)` - Location access with precision (tested across all platforms)
/// - `Microphone` - Microphone access (tested across all platforms)
/// - `Notifications` - Push notifications (tested on Android and Web)
/// - `Custom` - Custom permission with platform-specific identifiers
///
/// See the main documentation for examples of using `Custom` permissions
/// for untested or special use cases.
#[proc_macro]
pub fn static_permission(input: TokenStream) -> TokenStream {
    let permission = parse_macro_input!(input as PermissionParser);

    quote! { #permission }.into()
}

/// Backward compatible alias for [`static_permission!`].
#[proc_macro]
pub fn permission(input: TokenStream) -> TokenStream {
    static_permission(input)
}
