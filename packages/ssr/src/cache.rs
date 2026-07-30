//! Dioxus SSR uses the design of templates to cache as much as possible about the HTML a block of rsx can render.
//!
//! The structure of templates can tell us what segments are rendered where and lets us cache segments in the output string.
//!
//! For example, in this code, we can cache the whole render:
//! ```rust, no_run
//! use dioxus::prelude::*;
//! rsx! {
//!     div {
//!         "Hello world"
//!     }
//! };
//! ```
//! Because everything exists in the template, we can calculate the whole HTML for the template once and then reuse it.
//! ```html
//! <div>Hello world</div>
//! ```
//!
//! If the template is more complex, we can only cache the parts that are static. In this case, we can cache `<div width="100px">` and `</div>`, but not the child text.
//!
//! ```rust, no_run
//! use dioxus::prelude::*;
//! let dynamic = 123;
//! rsx! {
//!     div {
//!         width: "100px",
//!         "{dynamic}"
//!     }
//! };
//!```

use crate::renderer::{BOOL_ATTRS, str_truthy};
use dioxus_core::{StaticElement, StaticText, VNode, VNodeChild};
use std::{fmt::Write, ops::AddAssign};

#[derive(Debug)]
pub(crate) struct StringCache {
    pub segments: Vec<Segment>,
}

#[derive(Default)]
pub struct StringChain {
    // If we should add new static text to the last segment
    add_text_to_last_segment: bool,
    segments: Vec<Segment>,
}

impl StringChain {
    /// Add a new segment
    pub fn push(&mut self, segment: Segment) {
        self.add_text_to_last_segment = matches!(segment, Segment::PreRendered(_));
        self.segments.push(segment);
    }
}

impl AddAssign<Segment> for StringChain {
    fn add_assign(&mut self, rhs: Segment) {
        self.push(rhs)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// The escape text enum is used to mark segments that should be escaped
/// when rendering. This is used to prevent XSS attacks by escaping user input.
pub(crate) enum EscapeText {
    /// Always escape the text. This will be assigned if the text node is under
    /// a normal tag like a div in the template
    Escape,
    /// Don't escape the text. This will be assigned if the text node is under
    /// a script or style tag in the template
    NoEscape,
    /// Only escape the tag if this is rendered under a script or style tag in
    /// the parent template. This will be assigned if the text node is a root
    /// node in the template
    ParentEscape,
}

impl EscapeText {
    /// Check if the text should be escaped based on the parent's resolved
    /// escape text value
    pub fn should_escape(&self, parent_escaped: bool) -> bool {
        match self {
            EscapeText::Escape => true,
            EscapeText::NoEscape => false,
            EscapeText::ParentEscape => parent_escaped,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum Segment {
    /// A marker for where to insert the next dynamic attribute slot.
    Attr,
    /// A marker for where to insert the next dynamic node slot.
    Node { escape_text: EscapeText },
    /// Text that we know is static in the template that is pre-rendered
    PreRendered(String),
    /// Text we know is static in the template that is pre-rendered that may or may not be escaped
    PreRenderedMaybeEscaped {
        /// The text to render
        value: String,
        /// Only render this text if the escaped value is this
        renderer_if_escaped: bool,
    },
    /// A marker for where to insert a dynamic styles
    StyleMarker {
        // If the marker is inside a style tag or not
        // This will be true if there are static styles
        inside_style_tag: bool,
    },
    /// A marker for where to insert a dynamic inner html
    InnerHtmlMarker,
}

impl std::fmt::Write for StringChain {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        if self.add_text_to_last_segment {
            match self.segments.last_mut() {
                Some(Segment::PreRendered(s2)) => s2.push_str(s),
                _ => unreachable!(),
            }
        } else {
            self.segments.push(Segment::PreRendered(s.to_string()))
        }

        self.add_text_to_last_segment = true;

        Ok(())
    }
}

impl StringCache {
    /// Create a new string cache from a template
    pub fn from_template(vnode: &VNode) -> Result<Self, std::fmt::Error> {
        let mut chain = StringChain::default();

        for child in vnode.children() {
            from_template_child(child, EscapeText::ParentEscape, &mut chain)?;
        }

        Ok(Self {
            segments: chain.segments,
        })
    }
}

fn from_template_children(
    element: StaticElement<'_>,
    escape_text: EscapeText,
    chain: &mut StringChain,
) -> Result<(), std::fmt::Error> {
    for child in element.children() {
        from_template_child(child, escape_text, chain)?;
    }
    Ok(())
}

fn from_template_child(
    child: VNodeChild<'_>,
    escape_text: EscapeText,
    chain: &mut StringChain,
) -> Result<(), std::fmt::Error> {
    match child {
        VNodeChild::Element(element) => from_template_element(element, chain),
        VNodeChild::Text(text) => from_template_text(text, escape_text, chain),
        VNodeChild::Dynamic(anchor) => {
            for _ in anchor.nodes() {
                chain.push(Segment::Node { escape_text });
            }
            Ok(())
        }
    }
}

fn from_template_element(
    element: StaticElement<'_>,
    chain: &mut StringChain,
) -> Result<(), std::fmt::Error> {
    let tag = element.tag();
    write!(chain, "<{tag}")?;
    // we need to collect the styles and write them at the end
    let mut styles = Vec::new();
    // we need to collect the inner html and write it at the end
    let mut inner_html = None;
    for (name, value, namespace) in element.static_attributes() {
        if name == "dangerous_inner_html" {
            inner_html = Some(value);
        } else if let Some("style") = namespace {
            styles.push((name, value));
        } else if BOOL_ATTRS.contains(&name) {
            if str_truthy(value) {
                write!(
                    chain,
                    " {name}=\"{}\"",
                    askama_escape::escape(value, askama_escape::Html)
                )?;
            }
        } else {
            write!(
                chain,
                " {name}=\"{}\"",
                askama_escape::escape(value, askama_escape::Html)
            )?;
        }
    }

    // Dynamic attrs require style and inner-html placeholders even if those
    // dynamic attrs are not style or inner-html themselves.
    let has_dyn_attrs = emit_dynamic_attrs(element, chain);

    // write the styles
    if !styles.is_empty() {
        write!(chain, " style=\"")?;
        for (name, value) in styles {
            write!(
                chain,
                "{name}:{};",
                askama_escape::escape(value, askama_escape::Html)
            )?;
        }
        *chain += Segment::StyleMarker {
            inside_style_tag: true,
        };
        write!(chain, "\"")?;
    } else if has_dyn_attrs {
        *chain += Segment::StyleMarker {
            inside_style_tag: false,
        };
    }

    if !element.has_children() && tag_is_self_closing(tag) {
        write!(chain, "/>")?;
    } else {
        write!(chain, ">")?;
        // Write the static inner html, or insert a marker if dynamic inner html is possible
        if let Some(inner_html) = inner_html {
            chain.write_str(inner_html)?;
        } else if has_dyn_attrs {
            *chain += Segment::InnerHtmlMarker;
        }

        // Escape the text in children if this is not a style or script tag. If it is a style
        // or script tag, we want to allow the user to write code inside the tag
        let escape_text = match tag {
            "style" | "script" => EscapeText::NoEscape,
            _ => EscapeText::Escape,
        };

        // `pre`, `textarea`, and `listing` are "raw text" elements: when parsing
        // their content the HTML parser drops a single newline immediately after
        // the start tag. The serialization spec compensates by emitting an extra
        // leading newline whenever the content starts with one, so it round-trips.
        // Without this a leading `\n` is silently lost on standalone SSR and
        // desyncs the markerless hydration walk (which reconstructs text-node
        // offsets by length). See dioxus#5548.
        if raw_text_strips_leading_newline(element) {
            writeln!(chain)?;
        }

        from_template_children(element, escape_text, chain)?;
        write!(chain, "</{tag}>")?;
    }

    Ok(())
}

fn from_template_text(
    text: StaticText<'_>,
    escape_text: EscapeText,
    chain: &mut StringChain,
) -> Result<(), std::fmt::Error> {
    let text = text.text();
    match escape_text {
        // If we know this is statically escaped we can just write it out
        EscapeText::Escape => {
            write!(
                chain,
                "{}",
                askama_escape::escape(text, askama_escape::Html)
            )?;
        }
        // If we know this is statically not escaped we can just write it out
        EscapeText::NoEscape => {
            write!(chain, "{}", text)?;
        }
        // Otherwise, write out both versions and let the renderer decide which one to use
        // at runtime
        EscapeText::ParentEscape => {
            *chain += Segment::PreRenderedMaybeEscaped {
                value: text.to_string(),
                renderer_if_escaped: false,
            };
            *chain += Segment::PreRenderedMaybeEscaped {
                value: askama_escape::escape(text, askama_escape::Html).to_string(),
                renderer_if_escaped: true,
            };
        }
    }
    Ok(())
}

fn emit_dynamic_attrs(element: StaticElement<'_>, chain: &mut StringChain) -> bool {
    let mut has_dyn_attrs = false;
    for anchor in element.dynamic_anchors() {
        for _ in anchor.attrs() {
            *chain += Segment::Attr;
            has_dyn_attrs = true;
        }
    }
    has_dyn_attrs
}

/// Whether `element` is a "raw text" element (`pre`, `textarea`, `listing`)
/// whose first child is static text starting with a newline, so SSR must emit a
/// compensating leading newline (see the call site and dioxus#5548).
///
/// Only static leading text is handled; a dynamic first child whose runtime
/// value begins with `\n` is not compensated (it would need a render-time check).
fn raw_text_strips_leading_newline(element: StaticElement<'_>) -> bool {
    matches!(element.tag(), "pre" | "textarea" | "listing")
        && matches!(
            element.children().next(),
            Some(VNodeChild::Text(text)) if text.text().starts_with('\n')
        )
}

fn tag_is_self_closing(tag: &str) -> bool {
    matches!(
        tag,
        "area"
            | "base"
            | "br"
            | "col"
            | "embed"
            | "hr"
            | "img"
            | "input"
            | "link"
            | "meta"
            | "param"
            | "source"
            | "track"
            | "wbr"
    )
}
