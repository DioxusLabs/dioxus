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

use crate::renderer::{str_truthy, BOOL_ATTRS};
use dioxus_core::{TemplateAttribute, TemplateNode, VNode};
use std::{fmt::Write, ops::AddAssign};

#[derive(Debug)]
pub(crate) struct StringCache {
    pub segments: Vec<Segment>,
}

#[derive(Default)]
pub struct StringChain {
    // If we should add new static text to the last segment
    // This will be true if the last segment is a static text and the last text isn't part of a hydration only boundary
    add_text_to_last_segment: bool,
    segments: Vec<Segment>,
}

impl StringChain {
    /// Add segments but only when hydration is enabled
    fn if_hydration_enabled<O>(
        &mut self,
        during_prerender: impl FnOnce(&mut StringChain) -> O,
    ) -> O {
        // Insert a placeholder jump to the end of the hydration only segments
        let jump_index = self.segments.len();
        *self += Segment::HydrationOnlySection(0);
        let out = during_prerender(self);
        // Go back and fill in where the placeholder jump should skip to
        let after_hydration_only_section = self.segments.len();
        // Don't add any text to static text in the hydration only section. This would cause the text to be skipped during non-hydration renders
        self.add_text_to_last_segment = false;
        self.segments[jump_index] = Segment::HydrationOnlySection(after_hydration_only_section);
        out
    }

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
    /// A marker for where to insert an attribute with a given index
    Attr(usize),
    /// A marker for where to insert a node with a given index
    Node {
        index: usize,
        escape_text: EscapeText,
    },
    /// Text that we know is static in the template that is pre-rendered
    PreRendered(String),
    /// Text we know is static in the template that is pre-rendered that may or may not be escaped
    PreRenderedMaybeEscaped {
        /// The text to render
        value: String,
        /// Only render this text if the escaped value is this
        renderer_if_escaped: bool,
    },
    /// Anything between this and the segments at the index is only required for hydration. If you don't need to hydrate, you can safely skip to the section at the given index
    HydrationOnlySection(usize),
    /// A marker for where to insert a dynamic styles
    StyleMarker {
        // If the marker is inside a style tag or not
        // This will be true if there are static styles
        inside_style_tag: bool,
    },
    /// A marker for where to insert a dynamic inner html
    InnerHtmlMarker,
    /// A marker for where to insert a node id for an attribute
    AttributeNodeMarker,
    /// A marker for where to insert a node id for a root node
    RootNodeMarker,
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
    /// Create a new string cache from a template. This intentionally does not include any settings about the render mode (hydration or not) so that we can reuse the cache for both hydration and non-hydration renders.
    pub fn from_template(template: &VNode) -> Result<Self, std::fmt::Error> {
        let mut chain = StringChain::default();

        let mut cur_path = vec![];

        for (root_idx, root) in template.template.roots().iter().enumerate() {
            from_template_recursive(
                root,
                &mut cur_path,
                root_idx,
                true,
                EscapeText::ParentEscape,
                &mut chain,
            )?;
        }

        Ok(Self {
            segments: chain.segments,
        })
    }
}

fn from_template_recursive(
    root: &TemplateNode,
    cur_path: &mut Vec<usize>,
    root_idx: usize,
    is_root: bool,
    escape_text: EscapeText,
    chain: &mut StringChain,
) -> Result<(), std::fmt::Error> {
    match root {
        TemplateNode::Element {
            tag,
            attrs,
            children,
            ..
        } => {
            cur_path.push(root_idx);
            write!(chain, "<{tag}")?;
            // we need to collect the styles and write them at the end
            let mut styles = Vec::new();
            // we need to collect the inner html and write it at the end
            let mut inner_html = None;
            // we need to keep track of if we have dynamic attrs to know if we need to insert a style and inner_html marker
            let mut has_dyn_attrs = false;
            for attr in *attrs {
                match attr {
                    TemplateAttribute::Static {
                        name,
                        value,
                        namespace,
                    } => {
                        if *name == "dangerous_inner_html" {
                            inner_html = Some(value);
                        } else if let Some("style") = namespace {
                            styles.push((name, value));
                        } else if BOOL_ATTRS.contains(name) {
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
                    TemplateAttribute::Dynamic { id: index } => {
                        let index = *index;
                        *chain += Segment::Attr(index);
                        has_dyn_attrs = true
                    }
                }
            }

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

            // write the id if we are prerendering and this is either a root node or a node with a dynamic attribute
            if has_dyn_attrs || is_root {
                chain.if_hydration_enabled(|chain| {
                    write!(chain, " data-node-hydration=\"")?;
                    if has_dyn_attrs {
                        *chain += Segment::AttributeNodeMarker;
                    } else if is_root {
                        *chain += Segment::RootNodeMarker;
                    }
                    write!(chain, "\"")?;
                    std::fmt::Result::Ok(())
                })?;
            }

            if children.is_empty() && tag_is_self_closing(tag) {
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
                let escape_text = match *tag {
                    "style" | "script" => EscapeText::NoEscape,
                    _ => EscapeText::Escape,
                };

                for child in *children {
                    from_template_recursive(child, cur_path, root_idx, false, escape_text, chain)?;
                }
                write!(chain, "</{tag}>")?;
            }
            cur_path.pop();
        }
        TemplateNode::Text { text } => {
            // write the id if we are prerendering and this is a root node that may need to be removed in the future
            if is_root {
                chain.if_hydration_enabled(|chain| {
                    write!(chain, "<!--node-id")?;
                    *chain += Segment::RootNodeMarker;
                    write!(chain, "-->")?;
                    std::fmt::Result::Ok(())
                })?;
            }
            match escape_text {
                // If we know this is statically escaped we can just write it out
                // rsx! { div { "hello" } }
                EscapeText::Escape => {
                    write!(
                        chain,
                        "{}",
                        askama_escape::escape(text, askama_escape::Html)
                    )?;
                }
                // If we know this is statically not escaped we can just write it out
                // rsx! { script { "console.log('hello')" } }
                EscapeText::NoEscape => {
                    write!(chain, "{}", text)?;
                }
                // Otherwise, write out both versions and let the renderer decide which one to use
                // at runtime
                // rsx! { "console.log('hello')" }
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
            if is_root {
                chain.if_hydration_enabled(|chain| write!(chain, "<!--#-->"))?;
            }
        }
        TemplateNode::Dynamic { id: idx } => {
            *chain += Segment::Node {
                index: *idx,
                escape_text,
            }
        }
    }

    Ok(())
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
