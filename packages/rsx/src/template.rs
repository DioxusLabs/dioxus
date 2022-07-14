use dioxus_core::{
    OwnedTemplateNode, OwnedTemplateValue, TemplateAttribute, TemplateAttributeValue,
    TemplateElement, TemplateNodeId, TemplateNodeType, TextTemplate, TextTemplateSegment,
};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Expr, Ident, LitStr};

use crate::{BodyNode, ElementAttr, FormattedSegment, Segment};

struct TemplateElementBuilder {
    tag: String,
    attributes: Vec<TemplateAttributeBuilder>,
    children: Vec<TemplateNodeId>,
    listeners: Vec<usize>,
    parent: Option<TemplateNodeId>,
}

struct TemplateAttributeBuilder {
    name: String,
    value: TemplateAttributeValue<OwnedTemplateValue>,
}

enum TemplateNodeTypeBuilder {
    Element(TemplateElementBuilder),
    Text(TextTemplate<Vec<TextTemplateSegment<String>>, String>),
    Fragment(Vec<TemplateNodeId>),
    DynamicNode(usize),
}

struct TemplateNodeBuilder {
    id: TemplateNodeId,
    node_type: TemplateNodeTypeBuilder,
}

#[derive(Default)]
struct TemplateBuilder {
    nodes: Vec<TemplateNodeBuilder>,
    dynamic_context: DynamicTemplateContextBuilder,
}

impl TemplateBuilder {
    pub fn from_root(root: BodyNode) -> Self {
        let mut builder = Self::default();

        builder.build_node(root, None);

        builder
    }

    fn build_node(&mut self, node: BodyNode, parent: Option<TemplateNodeId>) -> TemplateNodeId {
        let id = TemplateNodeId(self.nodes.len());
        match node {
            BodyNode::Element(el) => {
                let mut attributes = Vec::new();
                let mut listeners = Vec::new();
                for attr in el.attributes {
                    match attr.attr {
                        ElementAttr::AttrText { name, value } => {
                            if let Some(static_value) = value.to_static() {
                                attributes.push(TemplateAttributeBuilder {
                                    name: name.to_string(),
                                    value: TemplateAttributeValue::Static(
                                        OwnedTemplateValue::Text(static_value),
                                    ),
                                })
                            } else {
                                attributes.push(TemplateAttributeBuilder {
                                    name: name.to_string(),
                                    value: TemplateAttributeValue::Dynamic(
                                        self.dynamic_context.add_attr(quote!(#value)),
                                    ),
                                })
                            }
                        }
                        ElementAttr::CustomAttrText { name, value } => {
                            if let Some(static_value) = value.to_static() {
                                attributes.push(TemplateAttributeBuilder {
                                    name: name.value(),
                                    value: TemplateAttributeValue::Static(
                                        OwnedTemplateValue::Text(static_value),
                                    ),
                                })
                            } else {
                                attributes.push(TemplateAttributeBuilder {
                                    name: name.value(),
                                    value: TemplateAttributeValue::Dynamic(
                                        self.dynamic_context.add_attr(quote!(#value)),
                                    ),
                                })
                            }
                        }
                        ElementAttr::AttrExpression { name, value } => {
                            attributes.push(TemplateAttributeBuilder {
                                name: name.to_string(),
                                value: TemplateAttributeValue::Dynamic(
                                    self.dynamic_context.add_attr(quote!(#value)),
                                ),
                            })
                        }
                        ElementAttr::CustomAttrExpression { name, value } => {
                            attributes.push(TemplateAttributeBuilder {
                                name: name.value(),
                                value: TemplateAttributeValue::Dynamic(
                                    self.dynamic_context.add_attr(quote!(#value)),
                                ),
                            })
                        }
                        ElementAttr::EventTokens { name, tokens } => {
                            listeners.push(self.dynamic_context.add_listener(name, tokens))
                        }
                    }
                }
                let children: Vec<_> = el
                    .children
                    .into_iter()
                    .map(|child| self.build_node(child, Some(id)))
                    .collect();

                self.nodes.push(TemplateNodeBuilder {
                    id,
                    node_type: TemplateNodeTypeBuilder::Element(TemplateElementBuilder {
                        tag: el.name.to_string(),
                        attributes,
                        children,
                        listeners,
                        parent,
                    }),
                })
            }
            BodyNode::Component(comp) => {
                self.nodes.push(TemplateNodeBuilder {
                    id,
                    node_type: TemplateNodeTypeBuilder::DynamicNode(
                        self.dynamic_context.add_node(BodyNode::Component(comp)),
                    ),
                });
            }
            BodyNode::Text(txt) => {
                let mut segments = Vec::new();

                for segment in txt.segments {
                    segments.push(match segment {
                        Segment::Literal(lit) => TextTemplateSegment::Static(lit),
                        Segment::Formatted(fmted) => {
                            TextTemplateSegment::Dynamic(self.dynamic_context.add_text(fmted))
                        }
                    })
                }

                self.nodes.push(TemplateNodeBuilder {
                    id,
                    node_type: TemplateNodeTypeBuilder::Text(TextTemplate::new(segments)),
                });
            }
            BodyNode::RawExpr(expr) => {
                self.nodes.push(TemplateNodeBuilder {
                    id,
                    node_type: TemplateNodeTypeBuilder::DynamicNode(
                        self.dynamic_context.add_node(BodyNode::RawExpr(expr)),
                    ),
                });
            }
        }
        id
    }
}

#[derive(Default)]
struct DynamicTemplateContextBuilder {
    nodes: Vec<BodyNode>,
    text: Vec<FormattedSegment>,
    attributes: Vec<TokenStream>,
    listeners: Vec<(String, Expr)>,
}

impl DynamicTemplateContextBuilder {
    fn add_node(&mut self, node: BodyNode) -> usize {
        let node_id = self.nodes.len();

        self.nodes.push(node);

        node_id
    }

    fn add_text(&mut self, text: FormattedSegment) -> usize {
        let text_id = self.text.len();

        self.text.push(text);

        text_id
    }

    fn add_attr(&mut self, attr: TokenStream) -> usize {
        let attr_id = self.attributes.len();

        self.attributes.push(attr);

        attr_id
    }

    fn add_listener(&mut self, name: Ident, listener: Expr) -> usize {
        let listener_id = self.listeners.len();

        self.listeners.push((name.to_string(), listener));

        listener_id
    }
}
