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
use dioxus_core::prelude::*;
use std::fmt::Write;

#[derive(Debug, Default)]
pub(crate) struct StringCache {
    // If we should add new static text to the last segment
    // This will be true if the last segment is a static text and the last text isn't part of a hydration only boundary
    add_text_to_last_segment: bool,
    pub segments: Vec<Segment>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) enum Segment {
    /// A marker for where to insert an attribute with a given index
    Attr(usize),
    /// A marker for where to insert a node with a given index
    Node(usize),
    /// Text that we know is static in the template that is pre-rendered
    PreRendered(String),
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

impl StringCache {
    /// Add segments but only when hydration is enabled
    fn if_hydration_enabled<O>(
        &mut self,
        during_prerender: impl FnOnce(&mut StringCache) -> O,
    ) -> O {
        // Insert a placeholder jump to the end of the hydration only segments
        let jump_index = self.segments.len();
        self.push(Segment::HydrationOnlySection(0));

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

    /// Create a new string cache from a template. This intentionally does not include any settings about the render mode (hydration or not) so that we can reuse the cache for both hydration and non-hydration renders.
    pub fn from_template(template: &VNode) -> Result<Self, std::fmt::Error> {
        let mut cache = StringCache::default();

        for (root_idx, root) in template.template.roots.iter().enumerate() {
            cache.from_template_recursive(root, true)?;
        }

        Ok(cache)
    }

    fn from_template_recursive(
        &mut self,
        root: &TemplateNode,
        is_root: bool,
    ) -> Result<(), std::fmt::Error> {
        match root {
            TemplateNode::Element {
                tag,
                attrs,
                children,
                ..
            } => {
                write!(self, "<{tag}")?;
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
                                    write!(self, " {name}=\"{value}\"",)?;
                                }
                            } else {
                                write!(self, " {name}=\"{value}\"")?;
                            }
                        }
                        TemplateAttribute::Dynamic { id } => {
                            self.push(Segment::Attr(*id));
                            has_dyn_attrs = true
                        }
                    }
                }

                // write the styles
                if !styles.is_empty() {
                    write!(self, " style=\"")?;
                    for (name, value) in styles {
                        write!(self, "{name}:{value};")?;
                    }
                    self.push(Segment::StyleMarker {
                        inside_style_tag: true,
                    });
                    write!(self, "\"")?;
                } else if has_dyn_attrs {
                    self.push(Segment::StyleMarker {
                        inside_style_tag: false,
                    });
                }

                // write the id if we are prerendering and this is either a root node or a node with a dynamic attribute
                if has_dyn_attrs || is_root {
                    self.if_hydration_enabled(|chain| {
                        write!(chain, " data-node-hydration=\"")?;
                        if has_dyn_attrs {
                            chain.push(Segment::AttributeNodeMarker);
                        } else if is_root {
                            chain.push(Segment::RootNodeMarker);
                        }
                        write!(chain, "\"")?;
                        std::fmt::Result::Ok(())
                    })?;
                }

                if children.is_empty() && tag_is_self_closing(tag) {
                    write!(self, "/>")?;
                } else {
                    write!(self, ">")?;
                    // Write the static inner html, or insert a marker if dynamic inner html is possible
                    if let Some(inner_html) = inner_html {
                        self.write_str(inner_html)?;
                    } else if has_dyn_attrs {
                        self.push(Segment::InnerHtmlMarker);
                    }

                    for child in *children {
                        self.from_template_recursive(child, false)?;
                    }

                    write!(self, "</{tag}>")?;
                }
            }
            TemplateNode::Text { text } => {
                // write the id if we are prerendering and this is a root node that may need to be removed in the future
                if is_root {
                    self.if_hydration_enabled(|chain| {
                        write!(chain, "<!--node-id")?;
                        chain.push(Segment::RootNodeMarker);
                        write!(chain, "-->")?;
                        std::fmt::Result::Ok(())
                    })?;
                }
                write!(self, "{}", askama_escape::escape(text, askama_escape::Html))?;
                if is_root {
                    self.if_hydration_enabled(|chain| write!(chain, "<!--#-->"))?;
                }
            }
            TemplateNode::Dynamic { id } => self.push(Segment::Node(*id)),
        }

        Ok(())
    }
}

impl std::fmt::Write for StringCache {
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
