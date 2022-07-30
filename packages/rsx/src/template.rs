use dioxus_core::{
    OwnedTemplateValue, TemplateAttributeValue, TemplateNodeId, TextTemplate, TextTemplateSegment,
};
use proc_macro2::TokenStream;
use quote::{quote, ToTokens, TokenStreamExt};
use syn::{Expr, Ident};

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
        let tag_ident = syn::parse_str::<Ident>(&tag).expect(&tag);
        tokens.append_all(quote! {
            TemplateElement::new(
                dioxus_elements::#tag_ident::TAG_NAME,
                dioxus_elements::#tag_ident::NAME_SPACE,
                &[#(#attributes),*],
                &[#(#children),*],
                &[#(#listeners),*],
                #parent,
            )
        })
    }
}

struct TemplateAttributeBuilder {
    element_tag: String,
    name: String,
    value: TemplateAttributeValue<OwnedTemplateValue>,
}

impl ToTokens for TemplateAttributeBuilder {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self {
            element_tag,
            name,
            value,
        } = self;
        let value = match value {
            TemplateAttributeValue::Static(val) => {
                let val = match val {
                    OwnedTemplateValue::Text(txt) => quote! {StaticTemplateValue::Text(#txt)},
                    OwnedTemplateValue::Float32(f) => quote! {StaticTemplateValue::Float32(#f)},
                    OwnedTemplateValue::Float64(f) => quote! {StaticTemplateValue::Float64(#f)},
                    OwnedTemplateValue::Int32(i) => quote! {StaticTemplateValue::Int32(#i)},
                    OwnedTemplateValue::Int64(i) => quote! {StaticTemplateValue::Int64(#i)},
                    OwnedTemplateValue::Uint32(u) => quote! {StaticTemplateValue::Uint32(#u)},
                    OwnedTemplateValue::Uint64(u) => quote! {StaticTemplateValue::Uint64(#u)},
                    OwnedTemplateValue::Bool(b) => quote! {StaticTemplateValue::Bool(#b)},
                    OwnedTemplateValue::Vec3Float(f1, f2, f3) => {
                        quote! {StaticTemplateValue::Vec3Float(#f1, #f2, #f3)}
                    }
                    OwnedTemplateValue::Vec3Int(f1, f2, f3) => {
                        quote! {StaticTemplateValue::Vec3Int(#f1, #f2, #f3)}
                    }
                    OwnedTemplateValue::Vec3Uint(f1, f2, f3) => {
                        quote! {StaticTemplateValue::Vec3Uint(#f1, #f2, #f3)}
                    }
                    OwnedTemplateValue::Vec4Float(f1, f2, f3, f4) => {
                        quote! {StaticTemplateValue::Vec4Float(#f1, #f2, #f3, #f4)}
                    }
                    OwnedTemplateValue::Vec4Int(f1, f2, f3, f4) => {
                        quote! {StaticTemplateValue::Vec4Int(#f1, #f2, #f3, #f4)}
                    }
                    OwnedTemplateValue::Vec4Uint(f1, f2, f3, f4) => {
                        quote! {StaticTemplateValue::Vec4Uint(#f1, #f2, #f3, #f4)}
                    }
                    OwnedTemplateValue::Bytes(b) => quote! {StaticTemplateValue::Bytes(&[#(#b),*])},
                };
                quote! {TemplateAttributeValue::Static(#val)}
            }
            TemplateAttributeValue::Dynamic(idx) => quote! {TemplateAttributeValue::Dynamic(#idx)},
        };
        let name = syn::parse_str::<Ident>(&name).expect(&name);
        let tag = syn::parse_str::<Ident>(&element_tag).expect(&element_tag);
        tokens.append_all(quote! {
            TemplateAttribute{
                attribute: dioxus_elements::#tag::#name,
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
                                    element_tag: el.name.to_string(),
                                    name: name.to_string(),
                                    value: TemplateAttributeValue::Static(
                                        OwnedTemplateValue::Text(static_value),
                                    ),
                                })
                            } else {
                                attributes.push(TemplateAttributeBuilder {
                                    element_tag: el.name.to_string(),
                                    name: name.to_string(),
                                    value: TemplateAttributeValue::Dynamic(
                                        self.dynamic_context.add_attr(quote!(AttributeValue::Text(__cx.bump().alloc(#value.to_string())))),
                                    ),
                                })
                            }
                        }
                        ElementAttr::CustomAttrText { name, value } => {
                            if let Some(static_value) = value.to_static() {
                                attributes.push(TemplateAttributeBuilder {
                                    element_tag: el.name.to_string(),
                                    name: name.value(),
                                    value: TemplateAttributeValue::Static(
                                        OwnedTemplateValue::Text(static_value),
                                    ),
                                })
                            } else {
                                attributes.push(TemplateAttributeBuilder {
                                    element_tag: el.name.to_string(),
                                    name: name.value(),
                                    value: TemplateAttributeValue::Dynamic(
                                        self.dynamic_context.add_attr(quote!(AttributeValue::Text(__cx.bump().alloc(#value.to_string())))),
                                    ),
                                })
                            }
                        }
                        ElementAttr::AttrExpression { name, value } => {
                            attributes.push(TemplateAttributeBuilder {
                                element_tag: el.name.to_string(),
                                name: name.to_string(),
                                value: TemplateAttributeValue::Dynamic(
                                    self.dynamic_context.add_attr(quote!(AttributeValue::Text(__cx.bump().alloc(#value.to_string())))),
                                ),
                            })
                        }
                        ElementAttr::CustomAttrExpression { name, value } => {
                            attributes.push(TemplateAttributeBuilder {
                                element_tag: el.name.to_string(),
                                name: name.value(),
                                value: TemplateAttributeValue::Dynamic(
                                    self.dynamic_context.add_attr(quote!(AttributeValue::Text(__cx.bump().alloc(#value.to_string())))),
                                ),
                            })
                        }
                        ElementAttr::EventTokens { name, tokens } => {
                            listeners.push(self.dynamic_context.add_listener(name, tokens))
                        }
                    }
                }
                self.nodes.push(TemplateNodeBuilder {
                    id,
                    node_type: TemplateNodeTypeBuilder::Element(TemplateElementBuilder {
                        tag: el.name.to_string(),
                        attributes,
                        children: Vec::new(),
                        listeners,
                        parent,
                    }),
                });

                let children: Vec<_> = el
                    .children
                    .into_iter()
                    .map(|child| self.build_node(child, Some(id)))
                    .collect();
                let parent = &mut self.nodes[id.0];
                if let TemplateNodeTypeBuilder::Element(element) = &mut parent.node_type {
                    element.children = children;
                }
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

        let mut node_mapping = vec![None; dynamic_context.nodes.len()];
        for n in nodes {
            match &n.node_type {
                TemplateNodeTypeBuilder::DynamicNode(idx) => node_mapping[*idx] = Some(n.id),
                _ => (),
            }
        }
        let node_mapping_quoted = node_mapping.iter().map(|op| match op {
            Some(id) => {
                let raw_id = id.0;
                quote! {Some(TemplateNodeId(#raw_id))}
            }
            None => quote! {None},
        });

        let mut text_mapping = vec![Vec::new(); dynamic_context.text.len()];
        for n in nodes {
            match &n.node_type {
                TemplateNodeTypeBuilder::Text(txt) => {
                    for seg in &txt.segments {
                        match seg {
                            TextTemplateSegment::Static(_) => (),
                            TextTemplateSegment::Dynamic(idx) => text_mapping[*idx].push(n.id),
                        }
                    }
                }
                _ => (),
            }
        }
        let text_mapping_quoted = text_mapping.iter().map(|inner| {
            let raw = inner.iter().map(|id| id.0);
            quote! {&[#(TemplateNodeId(#raw)),*]}
        });

        let mut attribute_mapping = vec![Vec::new(); dynamic_context.attributes.len()];
        for n in nodes {
            match &n.node_type {
                TemplateNodeTypeBuilder::Element(el) => {
                    for (i, attr) in el.attributes.iter().enumerate() {
                        match attr.value {
                            TemplateAttributeValue::Static(_) => (),
                            TemplateAttributeValue::Dynamic(idx) => {
                                attribute_mapping[idx].push((n.id, i));
                            }
                        }
                    }
                }
                _ => (),
            }
        }
        let attribute_mapping_quoted = attribute_mapping.iter().map(|inner| {
            let raw = inner.iter().map(|(id, _)| id.0);
            let indecies = inner.iter().map(|(_, idx)| idx);
            quote! {&[#((TemplateNodeId(#raw), #indecies)),*]}
        });

        let quoted = quote! {
            {
                const __NODES: dioxus::prelude::StaticTemplateNodes = &[#(#nodes),*];
                const __TEXT_MAPPING: &'static [&'static [dioxus::prelude::TemplateNodeId]] = &[#(#text_mapping_quoted),*];
                const __ATTRIBUTE_MAPPING: &'static [&'static [(dioxus::prelude::TemplateNodeId, usize)]] = &[#(#attribute_mapping_quoted),*];
                const __NODE_MAPPING: &'static [Option<dioxus::prelude::TemplateNodeId>] = &[#(#node_mapping_quoted),*];
                static __VOLITALE_MAPPING_INNER: dioxus::core::exports::once_cell::sync::Lazy<Vec<(dioxus::prelude::TemplateNodeId, usize)>> = dioxus::core::exports::once_cell::sync::Lazy::new(||{
                    // check each property to see if it is volatile
                    let mut volatile = Vec::new();
                    for n in __NODES {
                        if let TemplateNodeType::Element(el) = &n.node_type {
                            for (i, attr) in el.attributes.iter().enumerate() {
                                if attr.attribute.volatile {
                                    volatile.push((n.id, i));
                                }
                            }
                        }
                    }
                    volatile
                });
                static __VOLITALE_MAPPING: &'static dioxus::core::exports::once_cell::sync::Lazy<Vec<(dioxus::prelude::TemplateNodeId, usize)>> = &__VOLITALE_MAPPING_INNER;
                static __STATIC_VOLITALE_MAPPING: dioxus::prelude::LazyStaticVec<(dioxus::prelude::TemplateNodeId, usize)> = LazyStaticVec(__VOLITALE_MAPPING);
                static __TEMPLATE: dioxus::prelude::Template = Template::Static {
                    nodes: __NODES,
                    dynamic_mapping: StaticDynamicNodeMapping::new(__NODE_MAPPING, __TEXT_MAPPING, __ATTRIBUTE_MAPPING, __STATIC_VOLITALE_MAPPING),
                };

                __cx.template_ref(dioxus::prelude::TemplateId(get_line_num!()), __TEMPLATE.clone(), #dynamic_context)
            }
        };

        tokens.append_all(quoted)
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
        let listeners_names = self
            .listeners
            .iter()
            .map(|(n, _)| syn::parse_str::<Ident>(n).expect(n));
        let listeners_exprs = self.listeners.iter().map(|(_, e)| e);
        tokens.append_all(quote! {
            TemplateContext {
                nodes: __cx.bump().alloc([#(#nodes),*]),
                text_segments: __cx.bump().alloc([#(&*__cx.bump().alloc_str(&#text.to_string())),*]),
                attributes: __cx.bump().alloc([#(#attributes),*]),
                listeners: __cx.bump().alloc([#(dioxus_elements::on::#listeners_names(__cx, #listeners_exprs)),*]),
            }
        })
    }
}
