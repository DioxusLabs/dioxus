use std::any::{Any, TypeId};

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

// Open questions:
// - How do we handle type coercion for different sized component property integers?
// - Should non-string hot literals go through the centralized pool?
// - Should formatted strings be a runtime concept?

pub struct DynamicLiteralPool {
    dynamic_text: Box<[String]>,
}

impl DynamicLiteralPool {
    pub fn new(dynamic_text: Vec<String>) -> Self {
        Self {
            dynamic_text: dynamic_text.into_boxed_slice(),
        }
    }

    pub fn component_property<T: 'static>(
        &mut self,
        id: usize,
        hot_reload: &HotReloadedTemplate,
        _coherse_type: T,
    ) -> T {
        fn assert_type<T: 'static, T2: 'static>(t: T) -> T2 {
            *(Box::new(t) as Box<dyn Any>).downcast::<T2>().unwrap()
        }
        let type_id = TypeId::of::<T>();
        if type_id == TypeId::of::<String>() {
            if let Some(HotReloadLiteral::Fmted(segments)) = hot_reload.component_values.get(id) {
                assert_type(self.render_formatted(segments).to_string())
            } else {
                panic!("Expected a string component property");
            }
        } else if type_id == TypeId::of::<&'static str>() {
            if let Some(HotReloadLiteral::Fmted(segments)) = hot_reload.component_values.get(id) {
                assert_type(Box::leak(
                    self.render_formatted(segments).to_string().into_boxed_str(),
                ))
            } else {
                panic!("Expected a string component property");
            }
        } else if type_id == TypeId::of::<i64>() {
            if let Some(HotReloadLiteral::Int(i)) = hot_reload.component_values.get(id) {
                assert_type(*i)
            } else {
                panic!("Expected an i64 component property");
            }
        } else if type_id == TypeId::of::<i32>() {
            if let Some(HotReloadLiteral::Int(i)) = hot_reload.component_values.get(id) {
                assert_type(*i as i32)
            } else {
                panic!("Expected an i32 component property");
            }
        } else if type_id == TypeId::of::<i16>() {
            if let Some(HotReloadLiteral::Int(i)) = hot_reload.component_values.get(id) {
                assert_type(*i as i16)
            } else {
                panic!("Expected an i16 component property");
            }
        } else if type_id == TypeId::of::<i8>() {
            if let Some(HotReloadLiteral::Int(i)) = hot_reload.component_values.get(id) {
                assert_type(*i as i8)
            } else {
                panic!("Expected an i8 component property");
            }
        } else if type_id == TypeId::of::<u64>() {
            if let Some(HotReloadLiteral::Int(i)) = hot_reload.component_values.get(id) {
                assert_type(*i as u64)
            } else {
                panic!("Expected an u64 component property");
            }
        } else if type_id == TypeId::of::<u32>() {
            if let Some(HotReloadLiteral::Int(i)) = hot_reload.component_values.get(id) {
                assert_type(*i as u32)
            } else {
                panic!("Expected an u32 component property");
            }
        } else if type_id == TypeId::of::<u16>() {
            if let Some(HotReloadLiteral::Int(i)) = hot_reload.component_values.get(id) {
                assert_type(*i as u16)
            } else {
                panic!("Expected an u16 component property");
            }
        } else if type_id == TypeId::of::<u8>() {
            if let Some(HotReloadLiteral::Int(i)) = hot_reload.component_values.get(id) {
                assert_type(*i as u8)
            } else {
                panic!("Expected an u8 component property");
            }
        } else if type_id == TypeId::of::<f32>() {
            if let Some(HotReloadLiteral::Float(f)) = hot_reload.component_values.get(id) {
                assert_type(*f)
            } else {
                panic!("Expected an f32 component property");
            }
        } else if type_id == TypeId::of::<f64>() {
            if let Some(HotReloadLiteral::Float(f)) = hot_reload.component_values.get(id) {
                assert_type(*f)
            } else {
                panic!("Expected an f64 component property");
            }
        } else if type_id == TypeId::of::<bool>() {
            if let Some(HotReloadLiteral::Bool(b)) = hot_reload.component_values.get(id) {
                assert_type(*b)
            } else {
                panic!("Expected an bool component property");
            }
        } else {
            panic!("Unsupported component property type");
        }
    }

    pub fn render_formatted(&self, segments: &FmtedSegments) -> String {
        segments.render_with(&self.dynamic_text)
    }
}

pub struct DynamicValuePool {
    dynamic_attributes: Box<[Box<[Attribute]>]>,
    dynamic_nodes: Box<[DynamicNode]>,
    literal_pool: DynamicLiteralPool,
}

impl DynamicValuePool {
    pub fn new(
        dynamic_nodes: Vec<DynamicNode>,
        dynamic_attributes: Vec<Box<[Attribute]>>,
        literal_pool: DynamicLiteralPool,
    ) -> Self {
        Self {
            dynamic_attributes: dynamic_attributes.into_boxed_slice(),
            dynamic_nodes: dynamic_nodes.into_boxed_slice(),
            literal_pool,
        }
    }

    pub fn render_with(&mut self, hot_reload: &HotReloadedTemplate) -> VNode {
        // Get the node_paths from a depth first traversal of the template
        let node_paths = hot_reload.node_paths();
        let attr_paths = hot_reload.attr_paths();

        let template = Template {
            name: "",
            roots: hot_reload.roots,
            node_paths,
            attr_paths,
        };
        let key = hot_reload
            .key
            .as_ref()
            .map(|key| self.literal_pool.render_formatted(key));
        let dynamic_nodes = hot_reload
            .dynamic_nodes
            .iter()
            .map(|node| self.render_dynamic_node(node))
            .collect();
        let dynamic_attrs = hot_reload
            .dynamic_attributes
            .iter()
            .map(|attr| self.render_attribute(attr))
            .collect();

        VNode::new(key, template, dynamic_nodes, dynamic_attrs)
    }

    fn render_dynamic_node(&mut self, node: &HotReloadDynamicNode) -> DynamicNode {
        match node {
            // If the node is dynamic, take it from the pool and return it
            HotReloadDynamicNode::Dynamic(id) => self.dynamic_nodes[*id].clone(),
            // Otherwise, format the text node and return it
            HotReloadDynamicNode::Formatted(segments) => DynamicNode::Text(VText {
                value: self.literal_pool.render_formatted(segments),
            }),
        }
    }

    fn render_attribute(&mut self, attr: &HotReloadAttribute) -> Box<[Attribute]> {
        match attr {
            HotReloadAttribute::Spread(id) => self.dynamic_attributes[*id].clone(),
            HotReloadAttribute::Named(NamedAttribute {
                name,
                namespace,
                value,
            }) => Box::new([Attribute {
                name,
                namespace: *namespace,
                value: match value {
                    HotReloadAttributeValue::Literal(HotReloadLiteral::Fmted(segments)) => {
                        AttributeValue::Text(self.literal_pool.render_formatted(segments))
                    }
                    HotReloadAttributeValue::Literal(HotReloadLiteral::Float(f)) => {
                        AttributeValue::Float(*f)
                    }
                    HotReloadAttributeValue::Literal(HotReloadLiteral::Int(i)) => {
                        AttributeValue::Int(*i)
                    }
                    HotReloadAttributeValue::Literal(HotReloadLiteral::Bool(b)) => {
                        AttributeValue::Bool(*b)
                    }
                    HotReloadAttributeValue::Dynamic(id) => {
                        self.dynamic_attributes[*id][0].value.clone()
                    }
                },
                volatile: false,
            }]),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[serde(bound(deserialize = "'de: 'static"))]
pub struct HotReloadTemplateWithLocation {
    pub location: String,
    pub template: HotReloadedTemplate,
}

#[derive(Debug, PartialEq, Clone)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serialize", serde(bound(deserialize = "'de: 'static")))]
pub struct HotReloadedTemplate {
    key: Option<FmtedSegments>,
    dynamic_nodes: Vec<HotReloadDynamicNode>,
    dynamic_attributes: Vec<HotReloadAttribute>,
    component_values: Vec<HotReloadLiteral>,
    #[cfg_attr(
        feature = "serialize",
        serde(deserialize_with = "crate::nodes::deserialize_leaky")
    )]
    pub roots: &'static [TemplateNode],
}

impl HotReloadedTemplate {
    pub fn new(
        key: Option<FmtedSegments>,
        dynamic_nodes: Vec<HotReloadDynamicNode>,
        dynamic_attributes: Vec<HotReloadAttribute>,
        component_values: Vec<HotReloadLiteral>,
        roots: &'static [TemplateNode],
    ) -> Self {
        Self {
            key,
            dynamic_nodes,
            dynamic_attributes,
            component_values,
            roots,
        }
    }

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
pub enum HotReloadDynamicNode {
    Dynamic(usize),
    Formatted(FmtedSegments),
}

#[derive(Debug, PartialEq, Clone)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serialize", serde(bound(deserialize = "'de: 'static")))]
pub enum HotReloadAttribute {
    Spread(usize),
    Named(NamedAttribute),
}

#[derive(Debug, PartialEq, Clone)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct NamedAttribute {
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

    value: HotReloadAttributeValue,
}

impl NamedAttribute {
    pub fn new(
        name: &'static str,
        namespace: Option<&'static str>,
        value: HotReloadAttributeValue,
    ) -> Self {
        Self {
            name,
            namespace,
            value,
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serialize", serde(bound(deserialize = "'de: 'static")))]
pub enum HotReloadAttributeValue {
    Literal(HotReloadLiteral),
    Dynamic(usize),
}
