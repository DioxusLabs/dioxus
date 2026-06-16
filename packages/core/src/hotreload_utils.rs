use std::{
    any::{Any, TypeId},
    hash::{Hash, Hasher},
};

use crate::nodes::DynamicValue;
#[cfg(feature = "serialize")]
use crate::template::deserialize_string_leaky;
use crate::{Attribute, AttributeValue, DynamicNode, Template, VNode, VText};

#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[doc(hidden)]
#[derive(Debug, PartialEq, Clone)]
pub struct HotreloadedLiteral {
    pub name: String,
    pub value: HotReloadLiteral,
}

#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[doc(hidden)]
#[derive(Debug, PartialEq, Clone)]
pub enum HotReloadLiteral {
    Fmted(FmtedSegments),
    Float(f64),
    Int(i64),
    Bool(bool),
}

impl HotReloadLiteral {
    pub fn as_fmted(&self) -> Option<&FmtedSegments> {
        match self {
            Self::Fmted(segments) => Some(segments),
            _ => None,
        }
    }

    pub fn as_float(&self) -> Option<f64> {
        match self {
            Self::Float(f) => Some(*f),
            _ => None,
        }
    }

    pub fn as_int(&self) -> Option<i64> {
        match self {
            Self::Int(i) => Some(*i),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(b) => Some(*b),
            _ => None,
        }
    }
}

impl Hash for HotReloadLiteral {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Self::Fmted(segments) => segments.hash(state),
            Self::Float(f) => f.to_bits().hash(state),
            Self::Int(i) => i.hash(state),
            Self::Bool(b) => b.hash(state),
        }
    }
}

#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[doc(hidden)]
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
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

type StaticStr = &'static str;

#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[doc(hidden)]
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub enum FmtSegment {
    Literal {
        #[cfg_attr(
            feature = "serialize",
            serde(deserialize_with = "deserialize_string_leaky")
        )]
        value: StaticStr,
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
//     Template::new(ops, strings, dynamics),
//     dynamic_values,
// )

// Open questions:
// - How do we handle type coercion for different sized component property integers?
// - Should non-string hot literals go through the centralized pool?
// - Should formatted strings be a runtime concept?

#[doc(hidden)]
pub struct DynamicLiteralPool {
    dynamic_text: Box<[String]>,
}

impl DynamicLiteralPool {
    pub fn new(dynamic_text: Vec<String>) -> Self {
        Self {
            dynamic_text: dynamic_text.into_boxed_slice(),
        }
    }

    // TODO: This should be marked as private in the next major release
    pub fn get_component_property<'a, T>(
        &self,
        id: usize,
        hot_reload: &'a HotReloadedTemplate,
        f: impl FnOnce(&'a HotReloadLiteral) -> Option<T>,
    ) -> Option<T> {
        let value = hot_reload.component_values.get(id)?;
        f(value)
    }

    fn get_component_property_or_default<'a, T: Default>(
        &self,
        id: usize,
        hot_reload: &'a HotReloadedTemplate,
        f: impl FnOnce(&'a HotReloadLiteral) -> Option<T>,
    ) -> Option<T> {
        // If the component was removed since the last hot reload, the hot reload template may not
        // have the property. If that is the case, just use a default value since the component is
        // never rendered.
        if id >= hot_reload.component_values.len() {
            return Some(T::default());
        }
        self.get_component_property(id, hot_reload, f)
    }

    /// Get a component property of a specific type at the component property index
    pub fn component_property<T: 'static>(
        &mut self,
        id: usize,
        hot_reload: &HotReloadedTemplate,
        // We pass in the original value for better type inference
        // For example, if the original literal is `0i128`, we know the output must be the type `i128`
        _coherse_type: T,
    ) -> T {
        fn assert_type<T: 'static, T2: 'static>(t: T) -> T2 {
            *(Box::new(t) as Box<dyn Any>).downcast::<T2>().unwrap()
        }
        let grab_float = || {
            self.get_component_property_or_default(id, hot_reload, HotReloadLiteral::as_float).unwrap_or_else(|| {
                tracing::error!("Expected a float component property, because the type was {}. The CLI gave the hot reloading engine a type of {:?}. This is probably caused by a bug in dioxus hot reloading. Please report this issue.", std::any::type_name::<T>(), hot_reload.component_values.get(id));
                Default::default()
            })
        };
        let grab_int = || {
            self.get_component_property_or_default(id, hot_reload, HotReloadLiteral::as_int).unwrap_or_else(|| {
                tracing::error!("Expected a integer component property, because the type was {}. The CLI gave the hot reloading engine a type of {:?}. This is probably caused by a bug in dioxus hot reloading. Please report this issue.", std::any::type_name::<T>(), hot_reload.component_values.get(id));
                Default::default()
            })
        };
        let grab_bool = || {
            self.get_component_property_or_default(id, hot_reload, HotReloadLiteral::as_bool).unwrap_or_else(|| {
                tracing::error!("Expected a bool component property, because the type was {}. The CLI gave the hot reloading engine a type of {:?}. This is probably caused by a bug in dioxus hot reloading. Please report this issue.", std::any::type_name::<T>(), hot_reload.component_values.get(id));
                Default::default()
            })
        };
        let grab_fmted = || {
            self.get_component_property_or_default(id, hot_reload, |fmted| HotReloadLiteral::as_fmted(fmted).map(|segments| self.render_formatted(segments))).unwrap_or_else(|| {
                tracing::error!("Expected a string component property, because the type was {}. The CLI gave the hot reloading engine a type of {:?}. This is probably caused by a bug in dioxus hot reloading. Please report this issue.", std::any::type_name::<T>(), hot_reload.component_values.get(id));
                Default::default()
            })
        };
        match TypeId::of::<T>() {
            // Any string types that accept a literal
            _ if TypeId::of::<String>() == TypeId::of::<T>() => assert_type(grab_fmted()),
            _ if TypeId::of::<&str>() == TypeId::of::<T>() => {
                assert_type(Box::leak(grab_fmted().into_boxed_str()) as &'static str)
            }
            // Any integer types that accept a literal
            _ if TypeId::of::<i128>() == TypeId::of::<T>() => assert_type(grab_int() as i128),
            _ if TypeId::of::<i64>() == TypeId::of::<T>() => assert_type(grab_int()),
            _ if TypeId::of::<i32>() == TypeId::of::<T>() => assert_type(grab_int() as i32),
            _ if TypeId::of::<i16>() == TypeId::of::<T>() => assert_type(grab_int() as i16),
            _ if TypeId::of::<i8>() == TypeId::of::<T>() => assert_type(grab_int() as i8),
            _ if TypeId::of::<isize>() == TypeId::of::<T>() => assert_type(grab_int() as isize),
            _ if TypeId::of::<u128>() == TypeId::of::<T>() => assert_type(grab_int() as u128),
            _ if TypeId::of::<u64>() == TypeId::of::<T>() => assert_type(grab_int() as u64),
            _ if TypeId::of::<u32>() == TypeId::of::<T>() => assert_type(grab_int() as u32),
            _ if TypeId::of::<u16>() == TypeId::of::<T>() => assert_type(grab_int() as u16),
            _ if TypeId::of::<u8>() == TypeId::of::<T>() => assert_type(grab_int() as u8),
            _ if TypeId::of::<usize>() == TypeId::of::<T>() => assert_type(grab_int() as usize),
            // Any float types that accept a literal
            _ if TypeId::of::<f64>() == TypeId::of::<T>() => assert_type(grab_float()),
            _ if TypeId::of::<f32>() == TypeId::of::<T>() => assert_type(grab_float() as f32),
            // Any bool types that accept a literal
            _ if TypeId::of::<bool>() == TypeId::of::<T>() => assert_type(grab_bool()),
            _ => panic!("Unsupported component property type"),
        }
    }

    pub fn render_formatted(&self, segments: &FmtedSegments) -> String {
        segments.render_with(&self.dynamic_text)
    }
}
#[doc(hidden)]
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

    pub fn from_vnode(vnode: &VNode, literal_pool: DynamicLiteralPool) -> Self {
        let mut dynamic_nodes = Vec::new();
        let mut dynamic_attributes = Vec::new();

        for anchor in vnode.template.anchors_in_document_order() {
            for idx in anchor.values() {
                match &vnode.dynamic_values[idx] {
                    DynamicValue::Node(node) => dynamic_nodes.push(node.clone()),
                    DynamicValue::Attrs(attrs) => dynamic_attributes.push(attrs.clone()),
                }
            }
        }

        Self::new(dynamic_nodes, dynamic_attributes, literal_pool)
    }

    pub fn render_with(&mut self, hot_reload: &HotReloadedTemplate) -> VNode {
        let key = hot_reload
            .key
            .as_ref()
            .map(|key| self.literal_pool.render_formatted(key));
        let dynamic_values = hot_reload
            .dynamic_slots
            .iter()
            .map(|slot| match slot {
                HotReloadDynamicSlot::Node(id) => {
                    DynamicValue::Node(self.render_dynamic_node(&hot_reload.dynamic_nodes[*id]))
                }
                HotReloadDynamicSlot::Attribute(id) => {
                    DynamicValue::Attrs(self.render_attribute(&hot_reload.dynamic_attributes[*id]))
                }
            })
            .collect();

        VNode::new(key, hot_reload.template, dynamic_values)
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

    fn render_attribute(&mut self, attr: &HotReloadDynamicAttribute) -> Box<[Attribute]> {
        match attr {
            HotReloadDynamicAttribute::Dynamic(id) => self.dynamic_attributes[*id].clone(),
            HotReloadDynamicAttribute::Named(NamedAttribute {
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

#[doc(hidden)]
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct HotReloadTemplateWithLocation {
    pub key: TemplateGlobalKey,
    pub template: HotReloadedTemplate,
}

#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Hash, PartialOrd, Eq, Ord)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct TemplateGlobalKey {
    pub file: String,
    pub line: usize,
    pub column: usize,
    pub index: usize,
}

#[doc(hidden)]
#[derive(Debug, PartialEq, Clone)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize))]
pub struct HotReloadedTemplate {
    pub key: Option<FmtedSegments>,
    pub dynamic_nodes: Vec<HotReloadDynamicNode>,
    pub dynamic_attributes: Vec<HotReloadDynamicAttribute>,
    pub component_values: Vec<HotReloadLiteral>,
    dynamic_slots: Vec<HotReloadDynamicSlot>,
    template: Template,
}

impl HotReloadedTemplate {
    fn new(
        key: Option<FmtedSegments>,
        dynamic_nodes: Vec<HotReloadDynamicNode>,
        dynamic_attributes: Vec<HotReloadDynamicAttribute>,
        component_values: Vec<HotReloadLiteral>,
        template: Template,
        dynamic_slots: Vec<HotReloadDynamicSlot>,
    ) -> Self {
        Self {
            key,
            dynamic_nodes,
            dynamic_attributes,
            component_values,
            dynamic_slots,
            template,
        }
    }

    fn new_from_document_order(
        key: Option<FmtedSegments>,
        dynamic_nodes: Vec<HotReloadDynamicNode>,
        dynamic_attributes: Vec<HotReloadDynamicAttribute>,
        component_values: Vec<HotReloadLiteral>,
        template: Template,
        dynamic_slots: Vec<HotReloadDynamicSlot>,
    ) -> Self {
        let dynamic_slots = template.reorder_dynamic_values_from_document_order(dynamic_slots);
        Self::new(
            key,
            dynamic_nodes,
            dynamic_attributes,
            component_values,
            template,
            dynamic_slots,
        )
    }

    pub fn from_raw_ops(
        key: Option<FmtedSegments>,
        dynamic_nodes: Vec<HotReloadDynamicNode>,
        dynamic_attributes: Vec<HotReloadDynamicAttribute>,
        component_values: Vec<HotReloadLiteral>,
        raw_ops: &'static [crate::template::TemplateRawOp],
        dynamic_slots: Vec<HotReloadDynamicSlot>,
    ) -> Self {
        Self::new_from_document_order(
            key,
            dynamic_nodes,
            dynamic_attributes,
            component_values,
            Template::from_raw_ops(raw_ops),
            dynamic_slots,
        )
    }

    pub fn from_template(
        key: Option<FmtedSegments>,
        dynamic_nodes: Vec<HotReloadDynamicNode>,
        dynamic_attributes: Vec<HotReloadDynamicAttribute>,
        component_values: Vec<HotReloadLiteral>,
        template: Template,
        dynamic_slots: Vec<HotReloadDynamicSlot>,
    ) -> Self {
        Self::new_from_document_order(
            key,
            dynamic_nodes,
            dynamic_attributes,
            component_values,
            template,
            dynamic_slots,
        )
    }

    pub fn root_count(&self) -> usize {
        self.template.root_count()
    }

    pub fn ops(&self) -> &'static [crate::TemplateOp] {
        self.template.ops()
    }

    pub fn strings(&self) -> crate::StaticStringInterner {
        self.template.strings()
    }

    /// Classify a dynamic value index using the transmitted slot table.
    pub fn dynamic_is_node(&self, dynamic_idx: usize) -> bool {
        matches!(
            self.dynamic_slots.get(dynamic_idx),
            Some(HotReloadDynamicSlot::Node(_))
        )
    }

    /// Classify a dynamic value index using the transmitted slot table.
    pub fn dynamic_is_attr(&self, dynamic_idx: usize) -> bool {
        matches!(
            self.dynamic_slots.get(dynamic_idx),
            Some(HotReloadDynamicSlot::Attribute(_))
        )
    }
}

#[cfg(feature = "serialize")]
impl<'de> serde::Deserialize<'de> for HotReloadedTemplate {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        struct SerializedTemplate {
            #[serde(deserialize_with = "crate::template::deserialize_leaky")]
            ops: &'static [crate::TemplateOp],
            strings: crate::StaticStringInterner,
            #[serde(deserialize_with = "crate::template::deserialize_leaky")]
            anchors: &'static [crate::template::TemplateAnchor],
            hash: u64,
        }

        #[derive(serde::Deserialize)]
        struct SerializedHotReloadedTemplate {
            key: Option<FmtedSegments>,
            dynamic_nodes: Vec<HotReloadDynamicNode>,
            dynamic_attributes: Vec<HotReloadDynamicAttribute>,
            component_values: Vec<HotReloadLiteral>,
            dynamic_slots: Vec<HotReloadDynamicSlot>,
            template: SerializedTemplate,
        }

        let serialized = SerializedHotReloadedTemplate::deserialize(deserializer)?;
        let _serialized_hash = serialized.template.hash;
        Ok(Self::new(
            serialized.key,
            serialized.dynamic_nodes,
            serialized.dynamic_attributes,
            serialized.component_values,
            Template::new(
                serialized.template.ops,
                serialized.template.strings,
                serialized.template.anchors,
            ),
            serialized.dynamic_slots,
        ))
    }
}

#[doc(hidden)]
#[derive(Debug, PartialEq, Clone, Copy, Hash)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub enum HotReloadDynamicSlot {
    Node(usize),
    Attribute(usize),
}

#[doc(hidden)]
#[derive(Debug, PartialEq, Clone, Hash)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub enum HotReloadDynamicNode {
    Dynamic(usize),
    Formatted(FmtedSegments),
}

#[doc(hidden)]
#[derive(Debug, PartialEq, Clone, Hash)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub enum HotReloadDynamicAttribute {
    Dynamic(usize),
    Named(NamedAttribute),
}

#[doc(hidden)]
#[derive(Debug, PartialEq, Clone, Hash)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub struct NamedAttribute {
    /// The name of this attribute.
    #[cfg_attr(
        feature = "serialize",
        serde(deserialize_with = "crate::template::deserialize_string_leaky")
    )]
    name: StaticStr,
    /// The namespace of this attribute. Does not exist in the HTML spec
    #[cfg_attr(
        feature = "serialize",
        serde(deserialize_with = "crate::template::deserialize_option_leaky")
    )]
    namespace: Option<StaticStr>,

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

#[doc(hidden)]
#[derive(Debug, PartialEq, Clone, Hash)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub enum HotReloadAttributeValue {
    Literal(HotReloadLiteral),
    Dynamic(usize),
}
