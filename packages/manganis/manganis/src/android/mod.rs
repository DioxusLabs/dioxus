//! Android-specific utilities for mobile APIs

#[cfg(target_os = "android")]
pub mod activity;
#[cfg(target_os = "android")]
pub mod callback;
#[cfg(target_os = "android")]
pub mod java;
#[cfg(target_os = "android")]
pub mod metadata;

#[doc(hidden)]
pub mod macro_helpers {
    //! Helper functions for macro expansion
    //!
    //! These functions are used internally by the `android_plugin!()` macro
    //! and should not be used directly.

    pub use crate::macro_helpers::copy_bytes;
}

#[cfg(target_os = "android")]
pub use activity::*;
#[cfg(target_os = "android")]
pub use callback::*;
#[cfg(target_os = "android")]
pub use java::*;

#[cfg(target_os = "android")]
pub use metadata::*;
