//! Android-specific utilities for mobile APIs

pub mod activity;
pub mod callback;
pub mod java;
pub mod metadata;

pub use activity::*;
pub use callback::*;
pub use java::*;

#[cfg(feature = "metadata")]
pub use metadata::JavaSourceMetadata;
