use std::fmt;

/// Marker error type used to indicate an error is expected control-flow (not an actual failure).
///
/// This is intentionally defined in `dioxus-core` so integrations (like fullstack) can attach it as a
/// `source()` to their own error types without introducing dependency cycles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[doc(hidden)]
pub struct RedirectControlFlow;

impl fmt::Display for RedirectControlFlow {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("redirect control-flow")
    }
}

impl std::error::Error for RedirectControlFlow {}


