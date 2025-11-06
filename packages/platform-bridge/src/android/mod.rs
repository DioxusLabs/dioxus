//! Android-specific utilities for mobile APIs

pub mod activity;
pub mod callback;
pub mod java;
pub mod metadata;

#[doc(hidden)]
pub mod macro_helpers {
    //! Helper functions for macro expansion
    //!
    //! These functions are used internally by the `android_plugin!()` macro
    //! and should not be used directly.

    /// Copy a slice into a constant sized buffer at compile time
    pub const fn copy_bytes<const N: usize>(bytes: &[u8]) -> [u8; N] {
        let mut out = [0; N];
        let mut i = 0;
        while i < N {
            out[i] = bytes[i];
            i += 1;
        }
        out
    }
}

pub use activity::*;
pub use callback::*;
pub use java::*;

#[cfg(feature = "metadata")]
pub use metadata::JavaSourceMetadata;

// Re-export LinkerSymbol for use in generated macro code
#[cfg(feature = "metadata")]
pub use manganis_core::LinkerSymbol;
