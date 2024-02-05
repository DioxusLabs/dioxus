use dioxus_core::prelude::*;
use std::fmt::Write;

use crate::renderer::{str_truthy, BOOL_ATTRS};

#[derive(Debug)]
pub struct StringCache {
    pub segments: Vec<Segment>,
    pub template: Template,
}

#[derive(Default)]
pub struct StringChain {
    pub segments: Vec<Segment>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Segment {
    Attr(usize),
    Node(usize),
    PreRendered(String),
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
        match self.segments.last_mut() {
            Some(Segment::PreRendered(s2)) => s2.push_str(s),
            _ => self.segments.push(Segment::PreRendered(s.to_string())),
        }

        Ok(())
    }
}

impl StringCache {
    pub fn from_template(template: &VNode, prerender: bool) -> Result<Self, std::fmt::Error> {
        let mut chain = StringChain::default();

        let mut cur_path = vec![];

        for (root_idx, root) in template.template.get().roots.iter().enumerate() {
            Self::recurse(root, &mut cur_path, root_idx, true, prerender, &mut chain)?;
        }

        Ok(Self {
            segments: chain.segments,
            template: template.template.get(),
        })
    }

    fn recurse(
        root: &TemplateNode,
        cur_path: &mut Vec<usize>,
        root_idx: usize,
        is_root: bool,
        prerender: bool,
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
                                    write!(chain, " {name}=\"{value}\"",)?;
                                }
                            } else {
                                write!(chain, " {name}=\"{value}\"")?;
                            }
                        }
                        TemplateAttribute::Dynamic { id: index } => {
                            let index = *index;
                            chain.segments.push(Segment::Attr(index));
                            has_dyn_attrs = true
                        }
                    }
                }

                // write the styles
                if !styles.is_empty() {
                    write!(chain, " style=\"")?;
                    for (name, value) in styles {
                        write!(chain, "{name}:{value};")?;
                    }
                    chain.segments.push(Segment::StyleMarker {
                        inside_style_tag: true,
                    });
                    write!(chain, "\"")?;
                } else if has_dyn_attrs {
                    chain.segments.push(Segment::StyleMarker {
                        inside_style_tag: false,
                    });
                }

                // write the id if we are prerendering and this is either a root node or a node with a dynamic attribute
                if prerender && (has_dyn_attrs || is_root) {
                    write!(chain, " data-node-hydration=\"")?;
                    if has_dyn_attrs {
                        chain.segments.push(Segment::AttributeNodeMarker);
                    } else if is_root {
                        chain.segments.push(Segment::RootNodeMarker);
                    }
                    write!(chain, "\"")?;
                }

                if children.is_empty() && tag_is_self_closing(tag) {
                    write!(chain, "/>")?;
                } else {
                    write!(chain, ">")?;
                    // Write the static inner html, or insert a marker if dynamic inner html is possible
                    if let Some(inner_html) = inner_html {
                        chain.write_str(inner_html)?;
                    } else if has_dyn_attrs {
                        chain.segments.push(Segment::InnerHtmlMarker);
                    }

                    for child in *children {
                        Self::recurse(child, cur_path, root_idx, false, prerender, chain)?;
                    }
                    write!(chain, "</{tag}>")?;
                }
                cur_path.pop();
            }
            TemplateNode::Text { text } => {
                write!(
                    chain,
                    "{}",
                    askama_escape::escape(text, askama_escape::Html)
                )?;
            }
            TemplateNode::Dynamic { id: idx } | TemplateNode::DynamicText { id: idx } => {
                chain.segments.push(Segment::Node(*idx))
            }
        }

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
