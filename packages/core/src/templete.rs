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

/// A refrence to a template along with any context needed to hydrate it
pub(crate) struct VTemplateRef<'a> {
    pub(crate) id: Cell<Option<ElementId>>,
    pub(crate) template: &'static Template,
    pub(crate) dynamic_context: &'a TemplateContext<'a>,
}

#[derive(Debug)]
pub(crate) struct Template {
    pub(crate) id: TemplateId,
    pub(crate) root: &'static TemplateNode,
}

/// Templates can only contain a limited subset of VNodes and id/keys are not needed, as diffing will be skipped.
/// Dynamic parts of the Template are inserted into the VNode using the `TemplateContext` by traversing the tree in order and filling in dynamic parts
#[derive(Debug)]
pub enum TemplateNode {
    Element {
        tag: &'static str,
        namespace: Option<&'static str>,
        attributes: &'static [TemplateAttribute],
        children: &'static [TemplateNode],
        /// the number of listeners this node will have
        listeners: usize,
    },
    Text {
        text: TextTemplate,
    },
    Fragment {
        nodes: &'static [TemplateNode],
    },
    DynamicNode(usize),
}

impl TemplateNode {
    pub(crate) fn create<'b>(&self, mutations: &mut Mutations<'b>, bump: &'b Bump, id: ElementId) {
        mutations.create_templete(id.as_u64());
        let mut id = ElementId(0);
        self.create_inner(mutations, bump, &mut id);
        mutations.finish_templete();
    }

    fn create_inner<'b>(&self, mutations: &mut Mutations<'b>, bump: &'b Bump, id: &mut ElementId) {
        id.0 += 1;
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
                    child.create_inner(mutations, bump, id);
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
                    node.create_inner(mutations, bump, id);
                    id.0 += 1;
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct TextTemplate {
    // this is similar to what ifmt outputs and allows us to only diff the dynamic parts of the text
    segments: &'static [TextTemplateSegment],
}

#[derive(Debug)]
pub enum TextTemplateSegment {
    Static(&'static str),
    Dynamic(usize),
}

#[derive(Debug)]
pub struct TemplateAttribute {
    name: &'static str,
    namespace: Option<&'static str>,
    // if the attribute is dynamic, this will be empty
    value: TemplateAttributeValue,
}

#[derive(Debug)]
enum TemplateAttributeValue {
    Static(AttributeValue<'static>),
    Dynamic(usize),
}

struct TemplateContext<'a> {
    nodes: &'a [VNode<'a>],
    text_segments: &'a [&'a str],
    attributes: &'a [AttributeValue<'a>],
    listeners: &'a [Listener<'a>],
}
