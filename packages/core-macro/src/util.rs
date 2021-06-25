// use lazy_static::lazy_static;
use once_cell::sync::Lazy;
use std::collections::hash_set::HashSet;
use syn::{parse::ParseBuffer, Expr};

pub fn try_parse_bracketed(stream: &ParseBuffer) -> syn::Result<Expr> {
    let content;
    syn::braced!(content in stream);
    content.parse()
}

/// rsx! and html! macros support the html namespace as well as svg namespace
static HTML_TAGS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    [
        "a",
        "abbr",
        "address",
        "area",
        "article",
        "aside",
        "audio",
        "b",
        "base",
        "bdi",
        "bdo",
        "big",
        "blockquote",
        "body",
        "br",
        "button",
        "canvas",
        "caption",
        "cite",
        "code",
        "col",
        "colgroup",
        "command",
        "data",
        "datalist",
        "dd",
        "del",
        "details",
        "dfn",
        "dialog",
        "div",
        "dl",
        "dt",
        "em",
        "embed",
        "fieldset",
        "figcaption",
        "figure",
        "footer",
        "form",
        "h1",
        "h2",
        "h3",
        "h4",
        "h5",
        "h6",
        "head",
        "header",
        "hr",
        "html",
        "i",
        "iframe",
        "img",
        "input",
        "ins",
        "kbd",
        "keygen",
        "label",
        "legend",
        "li",
        "link",
        "main",
        "map",
        "mark",
        "menu",
        "menuitem",
        "meta",
        "meter",
        "nav",
        "noscript",
        "object",
        "ol",
        "optgroup",
        "option",
        "output",
        "p",
        "param",
        "picture",
        "pre",
        "progress",
        "q",
        "rp",
        "rt",
        "ruby",
        "s",
        "samp",
        "script",
        "section",
        "select",
        "small",
        "source",
        "span",
        "strong",
        "style",
        "sub",
        "summary",
        "sup",
        "table",
        "tbody",
        "td",
        "textarea",
        "tfoot",
        "th",
        "thead",
        "time",
        "title",
        "tr",
        "track",
        "u",
        "ul",
        "var",
        "video",
        "wbr",
    ]
    .iter()
    .cloned()
    .collect()
});

static SVG_TAGS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    [
        // SVTG
        "svg", "path", "g",
    ]
    .iter()
    .cloned()
    .collect()
});

// these tags are reserved by dioxus for any reason
// They might not all be used
static RESERVED_TAGS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    [
        // a fragment
        "fragment",
    ]
    .iter()
    .cloned()
    .collect()
});

/// Whether or not this tag is valid
///
/// ```
/// use html_validation::is_valid_tag;
///
/// assert_eq!(is_valid_tag("br"), true);
///
/// assert_eq!(is_valid_tag("random"), false);
/// ```
pub fn is_valid_tag(tag: &str) -> bool {
    is_valid_html_tag(tag) || is_valid_svg_tag(tag) || is_valid_reserved_tag(tag)
}

pub fn is_valid_html_tag(tag: &str) -> bool {
    HTML_TAGS.contains(tag)
}

pub fn is_valid_svg_tag(tag: &str) -> bool {
    SVG_TAGS.contains(tag)
}

pub fn is_valid_reserved_tag(tag: &str) -> bool {
    RESERVED_TAGS.contains(tag)
}
