/// Something that can check whether a string meets a condition.
///
/// This is used by matching routes (see the [`Segment`](super::Segment) `matching` function for
/// more details) to see if they are active.
pub trait Matcher: std::fmt::Debug {
    /// Check whether `segment_value` fulfills the [`Matcher`]s requirement.
    fn matches(&self, segment_value: &str) -> bool;
}

// The following implementation is for test purposes only. It could later be replaced by an
// implementation providing wildcard syntax or something similar.

#[cfg(test)]
impl Matcher for String {
    fn matches(&self, segment_value: &str) -> bool {
        self == segment_value
    }
}

#[cfg(feature = "regex")]
impl Matcher for regex::Regex {
    fn matches(&self, segment_value: &str) -> bool {
        self.is_match(segment_value)
    }
}
