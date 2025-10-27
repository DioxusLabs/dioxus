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
/// Basic permission declaration:
/// ```rust
/// use permissions_core::Permission;
/// use permissions_macro::static_permission;
/// const CAMERA: Permission = static_permission!(Camera, description = "Take photos");
/// ```
///
/// Location permission with precision:
/// ```rust
/// use permissions_core::Permission;
/// use permissions_macro::static_permission;
/// const LOCATION: Permission = static_permission!(Location(Fine), description = "Track your runs");
/// ```
///
/// Microphone permission:
/// ```rust
/// use permissions_core::Permission;
/// use permissions_macro::static_permission;
/// const MICROPHONE: Permission = static_permission!(Microphone, description = "Record audio");
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
/// - `Custom { ... }` - Custom permission with platform-specific identifiers
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
