use dioxus_core::prelude::*;
use std::fmt::Write;

#[derive(Debug)]
pub struct StringCache {
    pub segments: Vec<Segment>,
    pub template: Template<'static>,
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
    pub fn from_template(template: &VNode) -> Result<Self, std::fmt::Error> {
        let mut chain = StringChain::default();

        let mut cur_path = vec![];

        for (root_idx, root) in template.template.get().roots.iter().enumerate() {
            Self::recurse(root, &mut cur_path, root_idx, &mut chain)?;
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
                let mut has_dynamic_attrs = false;
                for attr in *attrs {
                    match attr {
                        TemplateAttribute::Static {
                            name,
                            value,
                            namespace,
                        } => {
                            if let Some("style") = namespace {
                                styles.push((name, value));
                            } else {
                                write!(chain, " {name}=\"{value}\"")?;
                            }
                        }
                        TemplateAttribute::Dynamic { id: index } => {
                            chain.segments.push(Segment::Attr(*index));
                            has_dynamic_attrs = true;
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
                } else if has_dynamic_attrs {
                    chain.segments.push(Segment::StyleMarker {
                        inside_style_tag: false,
                    });
                }

                if children.is_empty() && tag_is_self_closing(tag) {
                    write!(chain, "/>")?;
                } else {
                    write!(chain, ">")?;
                    for child in *children {
                        Self::recurse(child, cur_path, root_idx, chain)?;
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
