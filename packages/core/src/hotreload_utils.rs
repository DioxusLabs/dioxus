#[cfg(feature = "serialize")]
use crate::nodes::deserialize_string_leaky;
use crate::{
    Attribute, AttributeValue, DynamicNode, Template, TemplateAttribute, TemplateNode, VNode, VText,
};

#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serialize", serde(bound(deserialize = "'de: 'static")))]
#[derive(Debug, PartialEq, Clone)]
pub struct HotreloadedLiteral {
    pub name: String,
    pub value: HotReloadLiteral,
}

#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serialize", serde(bound(deserialize = "'de: 'static")))]
#[derive(Debug, PartialEq, Clone)]
pub enum HotReloadLiteral {
    Fmted(FmtedSegments),
    Float(f64),
    Int(i64),
    Bool(bool),
}

#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serialize", serde(bound(deserialize = "'de: 'static")))]
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct FmtedSegments {
    pub(crate) segments: Vec<FmtSegment>,
}

impl FmtedSegments {
    pub fn new(segments: Vec<FmtSegment>) -> Self {
        Self { segments }
    }

    /// Render the formatted string by stitching together the segments
    pub(crate) fn render_with(&self, dynamic_text: &[String]) -> String {
        let mut out = String::new();

        for segment in &self.segments {
            match segment {
                FmtSegment::Literal { value } => out.push_str(value),
                FmtSegment::Dynamic { id } => out.push_str(&dynamic_text[*id]),
            }
        }

        out
    }
}

#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum FmtSegment {
    Literal {
        #[cfg_attr(
            feature = "serialize",
            serde(deserialize_with = "deserialize_string_leaky")
        )]
        value: &'static str,
    },
    Dynamic {
        id: usize,
    },
}

// let __pool = DynamicValuePool::new(
//     vec![...],
//     vec![...],
//     vec![...],
// );
// VNode::new(
//     None,
//     Template {
//         name: "...",
//         roots: &[...],
//         node_paths: &[..],
//         attr_paths: &[...],
//     },
//     Box::new([...]),
//     Box::new([...]),
// )

pub struct DynamicValuePool {
    dynamic_attributes: Box<[Option<Box<[Attribute]>>]>,
    dynamic_nodes: Box<[Option<DynamicNode>]>,
    dynamic_text: Box<[String]>,
}

impl DynamicValuePool {
    pub fn new(
        dynamic_attributes: Vec<Box<[Attribute]>>,
        dynamic_nodes: Vec<DynamicNode>,
        dynamic_text: Box<[String]>,
    ) -> Self {
        Self {
            dynamic_attributes: dynamic_attributes.into_iter().map(Some).collect(),
            dynamic_nodes: dynamic_nodes.into_iter().map(Some).collect(),
            dynamic_text,
        }
    }

    fn render_with(&mut self, hot_reload: HotReloadedTemplate) -> VNode {
        // Get the node_paths from a depth first traversal of the template
        let node_paths = hot_reload.node_paths();
        let attr_paths = hot_reload.attr_paths();

        let template = Template {
            name: "",
            roots: hot_reload.roots,
            node_paths,
            attr_paths,
        };
        let key = hot_reload.key.map(|key| self.render_formatted(key));
        let dynamic_nodes = hot_reload
            .dynamic_nodes
            .into_iter()
            .map(|node| self.render_dynamic_node(node))
            .collect();
        let dynamic_attrs = hot_reload
            .dynamic_attributes
            .into_iter()
            .map(|attr| self.render_attribute(attr))
            .collect();

        VNode::new(key, template, dynamic_nodes, dynamic_attrs)
    }

    pub fn render_formatted(&self, segments: FmtedSegments) -> String {
        segments.render_with(&self.dynamic_text)
    }

    fn render_dynamic_node(&mut self, node: HotReloadDynamicNode) -> DynamicNode {
        match node {
            // If the node is dynamic, take it from the pool and return it
            HotReloadDynamicNode::Dynamic(id) => self.dynamic_nodes[id]
                .take()
                .expect("Hot reloaded nodes must only be taken once"),
            // Otherwise, format the text node and return it
            HotReloadDynamicNode::Formatted(segments) => DynamicNode::Text(VText {
                value: self.render_formatted(segments),
            }),
        }
    }

    fn render_attribute(&mut self, attr: HotReloadAttribute) -> Box<[Attribute]> {
        match attr {
            HotReloadAttribute::Dynamic(id) => self.dynamic_attributes[id]
                .take()
                .expect("Hot reloaded attributes must only be taken once"),
            HotReloadAttribute::Literal {
                name,
                namespace,
                value,
            } => Box::new([Attribute {
                name,
                namespace,
                value: match value {
                    HotReloadLiteral::Fmted(segments) => {
                        AttributeValue::Text(self.render_formatted(segments))
                    }
                    HotReloadLiteral::Float(f) => AttributeValue::Float(f),
                    HotReloadLiteral::Int(i) => AttributeValue::Int(i),
                    HotReloadLiteral::Bool(b) => AttributeValue::Bool(b),
                },
                volatile: false,
            }]),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serialize", serde(bound(deserialize = "'de: 'static")))]
struct HotReloadedTemplate {
    key: Option<FmtedSegments>,
    dynamic_nodes: Vec<HotReloadDynamicNode>,
    dynamic_attributes: Vec<HotReloadAttribute>,
    #[cfg_attr(
        feature = "serialize",
        serde(deserialize_with = "crate::nodes::deserialize_leaky")
    )]
    roots: &'static [TemplateNode],
}

impl HotReloadedTemplate {
    fn node_paths(&self) -> &'static [&'static [u8]] {
        fn add_node_paths(
            roots: &[TemplateNode],
            node_paths: &mut Vec<&'static [u8]>,
            current_path: Vec<u8>,
        ) {
            for (idx, node) in roots.iter().enumerate() {
                let mut path = current_path.clone();
                path.push(idx as u8);
                match node {
                    TemplateNode::Element { children, .. } => {
                        add_node_paths(children, node_paths, path);
                    }
                    TemplateNode::Text { .. } => {}
                    TemplateNode::Dynamic { id } => {
                        debug_assert_eq!(node_paths.len(), *id);
                        node_paths.push(Box::leak(path.into_boxed_slice()));
                    }
                }
            }
        }

        let mut node_paths = Vec::new();
        add_node_paths(self.roots, &mut node_paths, Vec::new());
        let leaked: &'static [&'static [u8]] = Box::leak(node_paths.into_boxed_slice());
        leaked
    }

    fn attr_paths(&self) -> &'static [&'static [u8]] {
        fn add_attr_paths(
            roots: &[TemplateNode],
            attr_paths: &mut Vec<&'static [u8]>,
            current_path: Vec<u8>,
        ) {
            for (idx, node) in roots.iter().enumerate() {
                let mut path = current_path.clone();
                path.push(idx as u8);
                if let TemplateNode::Element {
                    children, attrs, ..
                } = node
                {
                    for attr in *attrs {
                        if let TemplateAttribute::Dynamic { id } = attr {
                            debug_assert_eq!(attr_paths.len(), *id);
                            attr_paths.push(Box::leak(path.clone().into_boxed_slice()));
                        }
                    }
                    add_attr_paths(children, attr_paths, path);
                }
            }
        }

        let mut attr_paths = Vec::new();
        add_attr_paths(self.roots, &mut attr_paths, Vec::new());
        let leaked: &'static [&'static [u8]] = Box::leak(attr_paths.into_boxed_slice());
        leaked
    }
}

#[derive(Debug, PartialEq, Clone)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serialize", serde(bound(deserialize = "'de: 'static")))]
enum HotReloadDynamicNode {
    Dynamic(usize),
    Formatted(FmtedSegments),
}

#[derive(Debug, PartialEq, Clone)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
enum HotReloadAttribute {
    Dynamic(usize),
    Literal {
        /// The name of this attribute.
        #[cfg_attr(
            feature = "serialize",
            serde(deserialize_with = "crate::nodes::deserialize_string_leaky")
        )]
        name: &'static str,
        /// The namespace of this attribute. Does not exist in the HTML spec
        #[cfg_attr(
            feature = "serialize",
            serde(deserialize_with = "crate::nodes::deserialize_option_leaky")
        )]
        namespace: Option<&'static str>,

        value: HotReloadLiteral,
    },
}
