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
                for attr in *attrs {
                    match attr {
                        TemplateAttribute::Static { name, value, .. } => {
                            write!(chain, " {name}=\"{value}\"")?;
                        }
                        TemplateAttribute::Dynamic { id: index } => {
                            chain.segments.push(Segment::Attr(*index))
                        }
                    }
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
            TemplateNode::Text { text } => write!(chain, "{text}")?,
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
