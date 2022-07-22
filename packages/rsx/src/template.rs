use dioxus_core::{
    OwnedTemplateNode, OwnedTemplateValue, TemplateAttribute, TemplateAttributeValue,
    TemplateElement, TemplateNodeId, TemplateNodeType, TextTemplate, TextTemplateSegment,
};
use proc_macro2::TokenStream;
use quote::{quote, ToTokens, TokenStreamExt};
use syn::{Expr, Ident, LitStr};

use crate::{BodyNode, ElementAttr, FormattedSegment, Segment};

struct TemplateElementBuilder {
    tag: String,
    attributes: Vec<TemplateAttributeBuilder>,
    children: Vec<TemplateNodeId>,
    listeners: Vec<usize>,
    parent: Option<TemplateNodeId>,
}

impl ToTokens for TemplateElementBuilder {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self {
            tag,
            attributes,
            children,
            listeners,
            parent,
        } = self;
        let children = children.iter().map(|id| {
            let raw = id.0;
            quote! {TemplateNodeId(#raw)}
        });
        let parent = match parent {
            Some(id) => {
                let raw = id.0;
                quote! {Some(TemplateNodeId(#raw))}
            }
            None => quote! {None},
        };
        tokens.append_all(quote! {
            TemplateElement::new(
                tag: #tag,
                attributes: &[#(#attributes),*],
                children: &[#(#children),*],
                listeners: &[#(#listeners),*],
                parent: #parent,
            )
        })
    }
}

struct TemplateAttributeBuilder {
    name: String,
    value: TemplateAttributeValue<OwnedTemplateValue>,
}

impl ToTokens for TemplateAttributeBuilder {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self { name, value } = self;
        let value = match value {
            TemplateAttributeValue::Static(val) => {
                let val = match val {
                    OwnedTemplateValue::Text(txt) => quote! {AttributeValue::Text(#txt)},
                    OwnedTemplateValue::Float32(f) => quote! {AttributeValue::Float32(#f)},
                    OwnedTemplateValue::Float64(f) => quote! {AttributeValue::Float64(#f)},
                    OwnedTemplateValue::Int32(i) => quote! {AttributeValue::Int32(#i)},
                    OwnedTemplateValue::Int64(i) => quote! {AttributeValue::Int64(#i)},
                    OwnedTemplateValue::Uint32(u) => quote! {AttributeValue::Uint32(#u)},
                    OwnedTemplateValue::Uint64(u) => quote! {AttributeValue::Uint64(#u)},
                    OwnedTemplateValue::Bool(b) => quote! {AttributeValue::Bool(#b)},
                    OwnedTemplateValue::Vec3Float(f1, f2, f3) => {
                        quote! {AttributeValue::Vec3Float(#f1, #f2, #f3)}
                    }
                    OwnedTemplateValue::Vec3Int(f1, f2, f3) => {
                        quote! {AttributeValue::Vec3Int(#f1, #f2, #f3)}
                    }
                    OwnedTemplateValue::Vec3Uint(f1, f2, f3) => {
                        quote! {AttributeValue::Vec3Uint(#f1, #f2, #f3)}
                    }
                    OwnedTemplateValue::Vec4Float(f1, f2, f3, f4) => {
                        quote! {AttributeValue::Vec4Float(#f1, #f2, #f3, #f4)}
                    }
                    OwnedTemplateValue::Vec4Int(f1, f2, f3, f4) => {
                        quote! {AttributeValue::Vec4Int(#f1, #f2, #f3, #f4)}
                    }
                    OwnedTemplateValue::Vec4Uint(f1, f2, f3, f4) => {
                        quote! {AttributeValue::Vec4Uint(#f1, #f2, #f3, #f4)}
                    }
                    OwnedTemplateValue::Bytes(b) => quote! {AttributeValue::Bytes(&[#(#b),*])},
                };
                quote! {#val}
            }
            TemplateAttributeValue::Dynamic(idx) => quote! {AttributeValue::Dynamic(#idx)},
        };
        tokens.append_all(quote! {
            TemplateAttribute{
                attribute: dioxus_elements::#name,
                value: #value,
            }
        })
    }
}

enum TemplateNodeTypeBuilder {
    Element(TemplateElementBuilder),
    Text(TextTemplate<Vec<TextTemplateSegment<String>>, String>),
    Fragment(Vec<TemplateNodeId>),
    DynamicNode(usize),
}

impl ToTokens for TemplateNodeTypeBuilder {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            TemplateNodeTypeBuilder::Element(el) => tokens.append_all(quote! {
                TemplateNodeType::Element(#el)
            }),
            TemplateNodeTypeBuilder::Text(txt) => {
                let segments = txt.segments.iter().map(|seg| match seg {
                    TextTemplateSegment::Static(s) => quote!(TextTemplateSegment::Static(#s)),
                    TextTemplateSegment::Dynamic(idx) => quote!(TextTemplateSegment::Dynamic(#idx)),
                });
                tokens.append_all(quote! {
                    TemplateNodeType::Text(TextTemplate::new(&[#(#segments),*]))
                });
            }
            TemplateNodeTypeBuilder::Fragment(frag) => {
                let ids = frag.iter().map(|id| {
                    let raw = id.0;
                    quote! {TemplateNodeId(#raw)}
                });
                tokens.append_all(quote! {
                    TemplateNodeType::Fragment(&[#(#ids),*])
                });
            }
            TemplateNodeTypeBuilder::DynamicNode(idx) => tokens.append_all(quote! {
                TemplateNodeType::DynamicNode(#idx)
            }),
        }
    }
}

struct TemplateNodeBuilder {
    id: TemplateNodeId,
    node_type: TemplateNodeTypeBuilder,
}

impl ToTokens for TemplateNodeBuilder {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self { id, node_type } = self;
        let raw_id = id.0;

        tokens.append_all(quote! {
            TemplateNode {
                id: TemplateNodeId(#raw_id),
                node_type: #node_type,
            }
        })
    }
}

#[derive(Default)]
pub(crate) struct TemplateBuilder {
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

impl ToTokens for TemplateBuilder {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self {
            nodes,
            dynamic_context,
        } = self;
        tokens.append_all(quote! {
            Template {
                nodes: &[#(#nodes),*],
                dynamic_context: #dynamic_context,
            }
        })
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

impl ToTokens for DynamicTemplateContextBuilder {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let nodes = &self.nodes;
        let text = &self.text;
        let attributes = &self.attributes;
        let listeners_names = self.listeners.iter().map(|(n, _)| n);
        let listeners_exprs = self.listeners.iter().map(|(_, e)| e);
        tokens.append_all(quote! {
            TemplateContext {
                nodes: __cx.bump().alloc([#(#nodes),*]),
                text_segments: __cx.bump().alloc([#(__cx.bump().alloc(#text)),*]),
                attributes: __cx.bump().alloc([#(#attributes),*]),
                listeners: __cx.bump().alloc([#(dioxus_elements::on::#listeners_names(__cx.bump(), #listeners_exprs)),*]),
            }
        })
    }
}
