/// TODO @Jon
/// Figure out if validation should be its own crate, or embedded directly into dioxus
///
/// Should we even be bothered with validation?
///
///
///
mod validation {
    use once_cell::sync::Lazy;
    use std::collections::HashSet;

    // Used to uniquely identify elements that contain closures so that the DomUpdater can
    // look them up by their unique id.
    // When the DomUpdater sees that the element no longer exists it will drop all of it's
    // Rc'd Closures for those events.
    static SELF_CLOSING_TAGS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
        [
            "area", "base", "br", "col", "hr", "img", "input", "link", "meta", "param", "command",
            "keygen", "source",
        ]
        .iter()
        .cloned()
        .collect()
    });

    /// Whether or not this tag is self closing
    ///
    /// ```ignore
    /// use dioxus_core::validation::is_self_closing;
    /// assert_eq!(is_self_closing("br"), true);
    /// assert_eq!(is_self_closing("div"), false);
    /// ```
    pub fn is_self_closing(tag: &str) -> bool {
        SELF_CLOSING_TAGS.contains(tag)
        // SELF_CLOSING_TAGS.contains(tag) || is_self_closing_svg_tag(tag)
    }
}
