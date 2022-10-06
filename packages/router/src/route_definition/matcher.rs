use std::fmt::Debug;

use regex::Regex;

/// A trait that checks whether a matching route matches the segments current path.
///
/// # Example
/// ```rust
/// # extern crate dioxus_router;
/// # use dioxus_router::prelude::*;
/// #
/// #[derive(Debug)]
/// struct EvenMatcher;
///
/// impl SegmentMatch for EvenMatcher {
///     fn matches(&self, value: &str) -> bool {
///         value.len() % 2 == 0
///     }
/// }
/// ```
pub trait SegmentMatch: Debug {
    /// Check whether the `value` matches the requirements.
    fn matches(&self, value: &str) -> bool;
}

impl SegmentMatch for Regex {
    fn matches(&self, value: &str) -> bool {
        self.is_match(value)
    }
}
