use lazy_static::lazy_static;
use std::collections::hash_set::HashSet;

use super::svg_namespace::is_self_closing_svg_tag;

// Used to uniquely identify elements that contain closures so that the DomUpdater can
// look them up by their unique id.
// When the DomUpdater sees that the element no longer exists it will drop all of it's
// Rc'd Closures for those events.
lazy_static! {
    static ref SELF_CLOSING_TAGS: HashSet<&'static str> = [
        "area", "base", "br", "col", "hr", "img", "input", "link", "meta", "param", "command",
        "keygen", "source",
    ]
    .iter()
    .cloned()
    .collect();
}

/// Whether or not this tag is self closing
///
/// ```
/// use html_validation::is_self_closing;
///
/// assert_eq!(is_self_closing("br"), true);
///
/// assert_eq!(is_self_closing("div"), false);
/// ```
pub fn is_self_closing(tag: &str) -> bool {
    SELF_CLOSING_TAGS.contains(tag) || is_self_closing_svg_tag(tag)
}
