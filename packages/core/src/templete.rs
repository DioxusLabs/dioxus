use std::{cell::Cell, hash::Hash};

use bumpalo::Bump;

use crate::{Attribute, AttributeValue, ElementId, Listener, Mutations, VNode};

/// An Template's unique identifier.
///
/// `TemplateId` is a `usize` that is unique across the entire VirtualDOM and across time.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct TemplateId(pub usize);

impl TemplateId {
    pub fn as_u64(self) -> u64 {
        self.0 as u64
    }
}

/// A TemplateNode's unique identifier.
///
/// `TemplateNodeId` is a `usize` that is only unique across the template that contains it, it is not unique across multaple instances of that template.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct TemplateNodeId(pub usize);

impl TemplateId {
    pub fn as_u64(self) -> u64 {
        self.0 as u64
    }
}

/// A refrence to a template along with any context needed to hydrate it
pub(crate) struct VTemplateRef<'a> {
    pub(crate) id: Cell<Option<ElementId>>,
    pub(crate) template: &'static Template,
    pub(crate) dynamic_context: &'a TemplateContext<'a>,
}

#[derive(Debug)]
pub(crate) struct Template<'b> {
    pub(crate) id: TemplateId,
    pub(crate) nodes: &'b[TemplateNode<'b>],
    /// Any nodes that contain dynamic components. This is stored in the tmeplate to avoid traversing the tree every time a template is refrenced.
    pub(crate) dynamic_ids: &'b [TemplateNodeId],
}

/// Templates can only contain a limited subset of VNodes and id/keys are not needed, as diffing will be skipped.
/// Dynamic parts of the Template are inserted into the VNode using the `TemplateContext` by traversing the tree in order and filling in dynamic parts
#[derive(Debug)]
struct TemplateNode<'b>{
    /// The ID of the [`TemplateNode`]. Note that this is not an elenemt id, and should be allocated seperately from VNodes on the frontend.
    id: TemplateNodeId,
    node_type: TemplateNodeType<'b>
}

#[derive(Debug)]
pub enum TemplateNodeType<'b> {
    Element {
        tag: &'static str,
        namespace: Option<&'static str>,
        attributes: &'b [TemplateAttribute<'b>],
        children: &'b [TemplateNodeId],
        /// The index of each listener in the element
        listeners: &'b [usize],
    },
    Text {
        text: TextTemplate<'b>,
    },
    Fragment {
        nodes: &'b [TemplateNodeId],
    },
    /// The index in the dynamic node array this node should be replaced with
    DynamicNode(usize),
}

impl<'b> Template<'b> {
    pub(crate) fn create(&mut self, mutations: &mut Mutations<'b>, bump: &'b Bump, id: TemplateId) {
        mutations.create_templete(id.as_u64());
        let mut id = TemplateNodeId(0);
        if let Some(root) = self.nodes.get(id.0){
            self.create_node(mutations, bump, &mut id);
        }
        mutations.finish_templete();
    }
    
    fn create_node(&self, mutations: &mut Mutations<'b>, bump: &'b Bump, id: &mut TemplateNodeId) {
        match self {
            TemplateNode::Element {
                tag,
                namespace,
                attributes,
                children,
                listeners: _,
            } => {
                mutations.create_element(tag, *namespace, *id);
                for attr in *attributes {
                    if let TemplateAttributeValue::Static(val) = attr.value {
                        let val: AttributeValue<'b> = match val {
                            AttributeValue::Text(txt) => AttributeValue::Text(bump.alloc_str(txt)),
                            AttributeValue::Bytes(bytes) => {
                                AttributeValue::Bytes(bump.alloc_slice_copy(bytes))
                            }
                            AttributeValue::Float32(f) => AttributeValue::Float32(f),
                            AttributeValue::Float64(f) => AttributeValue::Float64(f),
                            AttributeValue::Int32(i) => AttributeValue::Int32(i),
                            AttributeValue::Int64(i) => AttributeValue::Int64(i),
                            AttributeValue::Uint32(u) => AttributeValue::Uint32(u),
                            AttributeValue::Uint64(u) => AttributeValue::Uint64(u),
                            AttributeValue::Bool(b) => AttributeValue::Bool(b),
                            AttributeValue::Vec3Float(f1, f2, f3) => {
                                AttributeValue::Vec3Float(f1, f2, f3)
                            }
                            AttributeValue::Vec3Int(i1, i2, i3) => {
                                AttributeValue::Vec3Int(i1, i2, i3)
                            }
                            AttributeValue::Vec3Uint(u1, u2, u3) => {
                                AttributeValue::Vec3Uint(u1, u2, u3)
                            }
                            AttributeValue::Vec4Float(f1, f2, f3, f4) => {
                                AttributeValue::Vec4Float(f1, f2, f3, f4)
                            }
                            AttributeValue::Vec4Int(i1, i2, i3, i4) => {
                                AttributeValue::Vec4Int(i1, i2, i3, i4)
                            }
                            AttributeValue::Vec4Uint(u1, u2, u3, u4) => {
                                AttributeValue::Vec4Uint(u1, u2, u3, u4)
                            }
                            AttributeValue::Any(a) => panic!("Any not supported"),
                        };
                        let attribute = Attribute {
                            name: attr.name,
                            value: val,
                            is_static: true,
                            is_volatile: false,
                            namespace: attr.namespace,
                        };
                        mutations.set_attribute(bump.alloc(attribute), id.as_u64());
                    }
                }
                for child in *children {
                    self.create_node(mutations, bump, child);
                }

                mutations.append_children(children.len() as u32)
            }
            TemplateNode::Text { text } => {
                if let &[TextTemplateSegment::Static(txt)] = text.segments {
                    mutations.create_text_node(txt, *id);
                } else {
                    mutations.create_text_node("", *id);
                }
            }
            TemplateNode::DynamicNode(_) => {
                mutations.create_placeholder(*id);
            }
            TemplateNode::Fragment { nodes } => {
                for node in *nodes {
                    self.create_node(mutations, bump, node);
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct TextTemplate<'b> {
    // this is similar to what ifmt outputs and allows us to only diff the dynamic parts of the text
    segments: &'b [TextTemplateSegment],
}

#[derive(Debug)]
pub enum TextTemplateSegment<'b> {
    Static(&'b str),
    Dynamic(usize),
}

#[derive(Debug)]
pub struct TemplateAttribute<'b> {
    name: &'static str,
    namespace: Option<&'static str>,
    // if the attribute is dynamic, this will be empty
    value: TemplateAttributeValue,
}

#[derive(Debug)]
enum TemplateAttributeValue<'b> {
    Static(AttributeValue<'b>),
    Dynamic(usize),
}

struct TemplateContext<'b> {
    nodes: &'b [VNode<'b>],
    text_segments: &'b [&'b str],
    attributes: &'b [AttributeValue<'b>],
    listeners: &'b [Listener<'b>],
}

impl<'b> TemplateContext<'b>{
    fn resolve_text(&self, text: &'b[TextTemplateSegment<'b>]) -> String{
        let mut result = String::new();
        for seg in text{
            match seg{
                TextTemplateSegment::Static(s) => result += s,
                TextTemplateSegment::Dynamic(idx) => result += self.text_segments[idx]
            }
        }
        result
    }

    fn resolve_attribute(&self, idx: usize) -> &'b AttributeValue<'b>{
        &self.attributes[idx]
    }

    fn resolve_listener(&self, idx: usize) -> &'b Listener<'b>{
        &self.listeners[idx]
    }
}
