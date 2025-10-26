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
/// - `Camera` - Camera access
/// - `Location(Fine)` / `Location(Coarse)` - Location access with precision
/// - `Microphone` - Microphone access
/// - `PhotoLibrary` - Photo library access
/// - `Contacts` - Contact list access
/// - `Calendar` - Calendar access
/// - `Bluetooth` - Bluetooth access
/// - `Notifications` - Push notifications
/// - `FileSystem` - File system access
/// - `Network` - Network access
/// - `Sms` - SMS access (Android only)
/// - `PhoneState` - Phone state access (Android only)
/// - `PhoneCall` - Phone call access (Android/Windows)
/// - `SystemAlertWindow` - System alert window (Android only)
/// - `UserTracking` - User tracking (iOS/macOS/Web)
/// - `FaceId` - Face ID access (iOS/macOS)
/// - `LocalNetwork` - Local network access (iOS/macOS)
/// - `Appointments` - Appointments access (Windows only)
/// - `WindowsPhoneCall` - Phone call access (Windows only)
/// - `EnterpriseAuth` - Enterprise authentication (Windows only)
/// - `Clipboard` - Clipboard access (Web only)
/// - `Payment` - Payment handling (Web only)
/// - `ScreenWakeLock` - Screen wake lock (Web only)
/// - `Custom { ... }` - Custom permission with platform-specific identifiers (not shown in doctests due to buffer size limitations)
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
