//! Platform-specific geolocation implementations

cfg_if::cfg_if! {
    if #[cfg(target_os = "android")] {
        mod android;
        pub use android::*;
    } else if #[cfg(target_os = "ios")] {
        mod ios;
        pub use ios::*;
    } else {
        mod unsupported;
        pub use unsupported::*;
    }
}
