use std::fmt::Debug;

use regex::Regex;

/// A trait that checks whether a matching route matches the segments current path.
pub trait SegmentMatch: Debug {
    /// Check whether the _value_ matches the requirements.
    fn matches(&self, value: &str) -> bool;
}

impl SegmentMatch for Regex {
    fn matches(&self, value: &str) -> bool {
        self.is_match(value)
    }
}
