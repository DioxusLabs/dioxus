use std::{convert::TryInto, panic::Location};

use dioxus_core::{
    prelude::TemplateNode, AttributeDiscription, CodeLocation, OwnedDynamicNodeMapping,
    OwnedTemplateNode, OwnedTemplateValue, Template, TemplateAttribute, TemplateAttributeValue,
    TemplateElement, TemplateNodeId, TemplateNodeType, TextTemplate, TextTemplateSegment,
};
use proc_macro2::TokenStream;
use quote::{quote, ToTokens, TokenStreamExt};
use syn::{Expr, Ident, LitStr};

use crate::{
    attributes::attrbute_to_static_str,
    elements::element_to_static_str,
    error::{Error, ParseError, RecompileReason},
    BodyNode, ElementAttr, FormattedSegment, Segment,
};

struct TemplateElementBuilder {
    tag: Ident,
    attributes: Vec<TemplateAttributeBuilder>,
    children: Vec<TemplateNodeId>,
    listeners: Vec<usize>,
    parent: Option<TemplateNodeId>,
}

impl TemplateElementBuilder {
    fn try_into_owned(
        self,
        location: &CodeLocation,
    ) -> Result<
        TemplateElement<
            Vec<TemplateAttribute<OwnedTemplateValue>>,
            OwnedTemplateValue,
            Vec<TemplateNodeId>,
            Vec<usize>,
        >,
        Error,
    > {
        let Self {
            tag,
            attributes,
            children,
            listeners,
            parent,
        } = self;
        let (element_tag, element_ns) =
            element_to_static_str(&tag.to_string()).ok_or_else(|| {
                Error::ParseError(ParseError::new(
                    syn::Error::new(tag.span(), format!("unknown element: {}", tag)),
                    location.clone(),
                ))
            })?;

        let mut owned_attributes = Vec::new();
        for a in attributes {
            owned_attributes.push(a.try_into_owned(location, element_tag, element_ns)?);
        }

        Ok(TemplateElement::new(
            element_tag,
            element_ns,
            owned_attributes,
            children,
            listeners,
            parent,
        ))
    }
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
                dioxus_elements::#tag::TAG_NAME,
                dioxus_elements::#tag::NAME_SPACE,
                &[#(#attributes),*],
                &[#(#children),*],
                &[#(#listeners),*],
                #parent,
            )
        })
    }
}

enum AttributeName {
    Ident(Ident),
    Str(LitStr),
}

struct TemplateAttributeBuilder {
    element_tag: Ident,
    name: AttributeName,
    value: TemplateAttributeValue<OwnedTemplateValue>,
}

impl TemplateAttributeBuilder {
    fn try_into_owned(
        self,
        location: &CodeLocation,
        element_tag: &'static str,
        element_ns: Option<&'static str>,
    ) -> Result<TemplateAttribute<OwnedTemplateValue>, Error> {
        let Self { name, value, .. } = self;
        let (name, span, literal) = match name {
            AttributeName::Ident(name) => (name.to_string(), name.span(), false),
            AttributeName::Str(name) => (name.value(), name.span(), true),
        };
        let (name, namespace, volatile) = attrbute_to_static_str(&name, element_tag, element_ns)
            .ok_or_else(|| {
                if literal {
                    // literals will be captured when a full recompile is triggered
                    Error::RecompileRequiredError(RecompileReason::CapturedAttribute(
                        name.to_string(),
                    ))
                } else {
                    Error::ParseError(ParseError::new(
                        syn::Error::new(span, format!("unknown attribute: {}", name)),
                        location.clone(),
                    ))
                }
            })?;
        let attribute = AttributeDiscription {
            name,
            namespace,
            volatile,
        };
        Ok(TemplateAttribute { value, attribute })
    }
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
        match name {
            AttributeName::Ident(name) => tokens.append_all(quote! {
                TemplateAttribute{
                    attribute: dioxus_elements::#element_tag::#name,
                    value: #value,
                }
            }),
            AttributeName::Str(lit) => tokens.append_all(quote! {
                TemplateAttribute{
                    attribute: dioxus::prelude::AttributeDiscription{
                        name: #lit,
                        namespace: None,
                        volatile: false
                    },
                    value: #value,
                }
            }),
        }
    }
}

enum TemplateNodeTypeBuilder {
    Element(TemplateElementBuilder),
    Text(TextTemplate<Vec<TextTemplateSegment<String>>, String>),
    DynamicNode(usize),
}

impl TemplateNodeTypeBuilder {
    fn try_into_owned(
        self,
        location: &CodeLocation,
    ) -> Result<
        TemplateNodeType<
            Vec<TemplateAttribute<OwnedTemplateValue>>,
            OwnedTemplateValue,
            Vec<TemplateNodeId>,
            Vec<usize>,
            Vec<TextTemplateSegment<String>>,
            String,
        >,
        Error,
    > {
        match self {
            TemplateNodeTypeBuilder::Element(el) => {
                Ok(TemplateNodeType::Element(el.try_into_owned(location)?))
            }
            TemplateNodeTypeBuilder::Text(txt) => Ok(TemplateNodeType::Text(txt)),
            TemplateNodeTypeBuilder::DynamicNode(idx) => Ok(TemplateNodeType::DynamicNode(idx)),
        }
    }
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

impl TemplateNodeBuilder {
    fn try_into_owned(self, location: &CodeLocation) -> Result<OwnedTemplateNode, Error> {
        let TemplateNodeBuilder { id, node_type } = self;
        let node_type = node_type.try_into_owned(location)?;
        Ok(OwnedTemplateNode {
            id,
            node_type,
            locally_static: false,
            fully_static: false,
        })
    }
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
    root_nodes: Vec<TemplateNodeId>,
    dynamic_context: DynamicTemplateContextBuilder,
}

impl TemplateBuilder {
    /// Create a template builder from nodes if it would improve performance to do so.
    pub fn from_roots(roots: Vec<BodyNode>) -> Option<Self> {
        let mut builder = Self::default();

        for root in roots {
            let id = builder.build_node(root, None);
            builder.root_nodes.push(id);
        }

        // only build a template if there is at least one static node
        if builder.nodes.iter().all(|r| {
            if let TemplateNodeTypeBuilder::DynamicNode(_) = &r.node_type {
                true
            } else {
                false
            }
        }) {
            None
        } else {
            Some(builder)
        }
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
                                    element_tag: el.name.clone(),
                                    name: AttributeName::Ident(name),
                                    value: TemplateAttributeValue::Static(
                                        OwnedTemplateValue::Text(static_value),
                                    ),
                                })
                            } else {
                                attributes.push(TemplateAttributeBuilder {
                                    element_tag: el.name.clone(),
                                    name: AttributeName::Ident(name),
                                    value: TemplateAttributeValue::Dynamic(
                                        self.dynamic_context.add_attr(quote!(AttributeValue::Text(__cx.bump().alloc(#value.to_string())))),
                                    ),
                                })
                            }
                        }
                        ElementAttr::CustomAttrText { name, value } => {
                            if let Some(static_value) = value.to_static() {
                                attributes.push(TemplateAttributeBuilder {
                                    element_tag: el.name.clone(),
                                    name: AttributeName::Str(name),
                                    value: TemplateAttributeValue::Static(
                                        OwnedTemplateValue::Text(static_value),
                                    ),
                                })
                            } else {
                                attributes.push(TemplateAttributeBuilder {
                                    element_tag: el.name.clone(),
                                    name: AttributeName::Str(name),
                                    value: TemplateAttributeValue::Dynamic(
                                        self.dynamic_context.add_attr(quote!(AttributeValue::Text(__cx.bump().alloc(#value.to_string())))),
                                    ),
                                })
                            }
                        }
                        ElementAttr::AttrExpression { name, value } => {
                            attributes.push(TemplateAttributeBuilder {
                                element_tag: el.name.clone(),
                                name: AttributeName::Ident(name),
                                value: TemplateAttributeValue::Dynamic(
                                    self.dynamic_context.add_attr(quote!(AttributeValue::Text(__cx.bump().alloc(#value.to_string())))),
                                ),
                            })
                        }
                        ElementAttr::CustomAttrExpression { name, value } => {
                            attributes.push(TemplateAttributeBuilder {
                                element_tag: el.name.clone(),
                                name: AttributeName::Str(name),
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
                        tag: el.name,
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

    fn try_into_owned(self, location: &CodeLocation) -> Result<Template, Error> {
        let dynamic_context = self.dynamic_context;
        let mut node_mapping = vec![None; dynamic_context.nodes.len()];
        let nodes = &self.nodes;
        for n in nodes {
            match &n.node_type {
                TemplateNodeTypeBuilder::DynamicNode(idx) => node_mapping[*idx] = Some(n.id),
                _ => (),
            }
        }
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
        let mut listener_mapping = Vec::new();
        for n in nodes {
            match &n.node_type {
                TemplateNodeTypeBuilder::Element(el) => {
                    if !el.listeners.is_empty() {
                        listener_mapping.push(n.id);
                    }
                }
                _ => (),
            }
        }
        let mut nodes = Vec::new();
        for node in self.nodes {
            nodes.push(node.try_into_owned(location)?);
        }

        let mut volatile_mapping = Vec::new();
        for n in &nodes {
            if let TemplateNodeType::Element(el) = &n.node_type {
                for (i, attr) in el.attributes.iter().enumerate() {
                    if attr.attribute.volatile {
                        volatile_mapping.push((n.id, i));
                    }
                }
            }
        }

        Ok(Template::Owned {
            nodes,
            root_nodes: self.root_nodes,
            dynamic_mapping: OwnedDynamicNodeMapping::new(
                node_mapping,
                text_mapping,
                attribute_mapping,
                volatile_mapping,
                listener_mapping,
            ),
        })
    }
}

impl ToTokens for TemplateBuilder {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let Self {
            nodes,
            root_nodes,
            dynamic_context,
        } = self;

        let mut node_mapping = vec![None; dynamic_context.nodes.len()];
        for n in nodes {
            match &n.node_type {
                TemplateNodeTypeBuilder::DynamicNode(idx) => node_mapping[*idx] = Some(n.id),
                _ => (),
            }
        }
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
        let mut listener_mapping = Vec::new();
        for n in nodes {
            match &n.node_type {
                TemplateNodeTypeBuilder::Element(el) => {
                    if !el.listeners.is_empty() {
                        listener_mapping.push(n.id);
                    }
                }
                _ => (),
            }
        }

        let root_nodes = root_nodes.iter().map(|id| {
            let raw = id.0;
            quote! { TemplateNodeId(#raw) }
        });
        let node_mapping_quoted = node_mapping.iter().map(|op| match op {
            Some(id) => {
                let raw_id = id.0;
                quote! {Some(TemplateNodeId(#raw_id))}
            }
            None => quote! {None},
        });
        let text_mapping_quoted = text_mapping.iter().map(|inner| {
            let raw = inner.iter().map(|id| id.0);
            quote! {&[#(TemplateNodeId(#raw)),*]}
        });
        let attribute_mapping_quoted = attribute_mapping.iter().map(|inner| {
            let raw = inner.iter().map(|(id, _)| id.0);
            let indecies = inner.iter().map(|(_, idx)| idx);
            quote! {&[#((TemplateNodeId(#raw), #indecies)),*]}
        });
        let listener_mapping_quoted = listener_mapping.iter().map(|id| {
            let raw = id.0;
            quote! {TemplateNodeId(#raw)}
        });

        let quoted = quote! {
            {
                const __NODES: dioxus::prelude::StaticTemplateNodes = &[#(#nodes),*];
                const __TEXT_MAPPING: &'static [&'static [dioxus::prelude::TemplateNodeId]] = &[#(#text_mapping_quoted),*];
                const __ATTRIBUTE_MAPPING: &'static [&'static [(dioxus::prelude::TemplateNodeId, usize)]] = &[#(#attribute_mapping_quoted),*];
                const __ROOT_NODES: &'static [dioxus::prelude::TemplateNodeId] = &[#(#root_nodes),*];
                const __NODE_MAPPING: &'static [Option<dioxus::prelude::TemplateNodeId>] = &[#(#node_mapping_quoted),*];
                const __NODES_WITH_LISTENERS: &'static [dioxus::prelude::TemplateNodeId] = &[#(#listener_mapping_quoted),*];
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
                    root_nodes: __ROOT_NODES,
                    dynamic_mapping: StaticDynamicNodeMapping::new(__NODE_MAPPING, __TEXT_MAPPING, __ATTRIBUTE_MAPPING, __STATIC_VOLITALE_MAPPING, __NODES_WITH_LISTENERS),
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
