use crate::innerlude::*;
use proc_macro2::{Ident, TokenStream as TokenStream2};
use quote::{format_ident, quote, quote_spanned};

#[derive(Clone, Debug)]
struct TypedView {
    expr: TokenStream2,
    child_expr: TokenStream2,
    ty: TokenStream2,
}

#[derive(Clone, Debug)]
struct TypedTag {
    expr: TokenStream2,
    ty: TokenStream2,
}

#[derive(Clone, Debug)]
struct TypedAttr {
    append: AttrAppend,
    ty: TokenStream2,
}

#[derive(Clone, Debug)]
enum AttrAppend {
    Direct(TokenStream2),
    Method { method: Ident, value: TokenStream2 },
}

impl AttrAppend {
    fn apply_to(self, target: TokenStream2) -> TokenStream2 {
        match self {
            Self::Direct(attr) => quote! { #target.attr(#attr) },
            Self::Method { method, value } => quote! { #target.#method(#value) },
        }
    }
}

/// Typed view-builder pieces collected from an RSX template.
#[derive(Clone, Debug)]
pub(crate) struct ViewBuilderPieces {
    definitions: Vec<TokenStream2>,
    view: TypedView,
    dynamic_text_tokens: Vec<TokenStream2>,
    component_value_tokens: Vec<TokenStream2>,
    hot_reload_dynamic_nodes: Vec<TokenStream2>,
    hot_reload_dynamic_attrs: Vec<TokenStream2>,
    hot_reload_key: Option<TokenStream2>,
}

impl ViewBuilderPieces {
    pub(crate) fn from_body(body: &TemplateBody) -> Self {
        let mut builder = ViewBuilder::new();
        let view = builder.visit_roots(&body.roots);
        Self {
            definitions: builder.definitions,
            view,
            dynamic_text_tokens: builder.dynamic_text_tokens,
            component_value_tokens: builder.component_value_tokens,
            hot_reload_dynamic_nodes: builder.hot_reload_dynamic_nodes,
            hot_reload_dynamic_attrs: builder.hot_reload_dynamic_attrs,
            hot_reload_key: builder.hot_reload_key,
        }
    }

    pub(crate) fn definitions(&self) -> impl Iterator<Item = &TokenStream2> {
        self.definitions.iter()
    }

    pub(crate) fn view_expr(&self) -> &TokenStream2 {
        &self.view.expr
    }

    pub(crate) fn dynamic_text_tokens(&self) -> &[TokenStream2] {
        &self.dynamic_text_tokens
    }

    pub(crate) fn hot_reload_template_tokens(&self, template: TokenStream2) -> TokenStream2 {
        let key = self
            .hot_reload_key
            .as_ref()
            .map(|key| quote! { Some(#key) })
            .unwrap_or_else(|| quote! { None });
        let dynamic_nodes = self.hot_reload_dynamic_nodes.iter();
        let dyn_attrs = self.hot_reload_dynamic_attrs.iter();
        let component_values = self.component_value_tokens.iter();

        quote! {
            dioxus_core::internal::HotReloadedTemplate::from_template(
                #key,
                vec![ #( #dynamic_nodes ),* ],
                vec![ #( #dyn_attrs ),* ],
                vec![ #( #component_values ),* ],
                #template,
            )
        }
    }
}

struct ViewBuilder {
    definitions: Vec<TokenStream2>,
    dynamic_node_count: usize,
    dynamic_attr_count: usize,
    dynamic_text_tokens: Vec<TokenStream2>,
    component_value_tokens: Vec<TokenStream2>,
    hot_reload_dynamic_nodes: Vec<TokenStream2>,
    hot_reload_dynamic_attrs: Vec<TokenStream2>,
    hot_reload_key: Option<TokenStream2>,
    next_marker: usize,
}

impl ViewBuilder {
    fn new() -> Self {
        Self {
            definitions: Vec::new(),
            dynamic_node_count: 0,
            dynamic_attr_count: 0,
            dynamic_text_tokens: Vec::new(),
            component_value_tokens: Vec::new(),
            hot_reload_dynamic_nodes: Vec::new(),
            hot_reload_dynamic_attrs: Vec::new(),
            hot_reload_key: None,
            next_marker: 0,
        }
    }

    fn visit_roots(&mut self, nodes: &[BodyNode]) -> TypedView {
        let roots = nodes
            .iter()
            .enumerate()
            .map(|(index, node)| self.visit_node(node, index == 0))
            .collect::<Vec<_>>();
        Self::tuple_or_unit(roots)
    }

    fn visit_node(&mut self, node: &BodyNode, implicit_key: bool) -> TypedView {
        match node {
            BodyNode::Element(element) => self.visit_element(element, implicit_key),
            BodyNode::Text(text) if text.is_static() => self.static_text(text),
            BodyNode::Text(text) => {
                self.allocate_formatted(&text.input);
                self.dynamic_node(quote! { #text })
            }
            BodyNode::Component(component) => {
                let literal_ids = self.component_literal_ids(component, implicit_key);
                self.dynamic_node(component.to_tokens_with_literal_ids(&literal_ids))
            }
            BodyNode::RawExpr(_) | BodyNode::ForLoop(_) | BodyNode::IfChain(_) => {
                self.dynamic_node(quote! { #node })
            }
        }
    }

    fn visit_element(&mut self, element: &Element, implicit_key: bool) -> TypedView {
        let tag = self.element_tag(element);
        let mut attrs = Vec::new();
        for attr in &element.merged_attributes {
            attrs.push(element.typed_builder_attribute(attr, self));
        }

        if let Some(AttributeValue::AttrLiteral(HotLiteral::Fmted(key))) = element.key() {
            let key = self.allocate_formatted(key);
            if implicit_key {
                self.hot_reload_key = Some(key);
            }
        }

        let children = element
            .children
            .iter()
            .map(|child| self.visit_node(child, false))
            .collect::<Vec<_>>();

        let attrs_ty = Self::fold_builder_tuple_type(attrs.iter().map(|attr| attr.ty.clone()));
        let children_ty =
            Self::fold_builder_tuple_type(children.iter().map(|child| child.ty.clone()));
        let tag_ty = tag.ty;
        let ty = quote! { dioxus_core::view::El<#tag_ty, #attrs_ty, #children_ty> };

        let mut child_definitions = Vec::new();
        let mut child_idents = Vec::new();
        for child in &children {
            let child_ident = self.next_ident("__DioxusChild");
            let child = child.child_expr.clone();
            child_definitions.push(quote! {
                let #child_ident = #child;
            });
            child_idents.push(child_ident);
        }

        let mut expr = tag.expr;
        for attr in attrs {
            expr = attr.append.apply_to(expr);
        }
        for child in child_idents {
            expr = quote! { #expr.child(#child) };
        }
        let expr = quote! {{
            #(#child_definitions)*
            #expr
        }};

        TypedView {
            child_expr: expr.clone(),
            expr,
            ty,
        }
    }

    fn static_text(&mut self, text: &TextNode) -> TypedView {
        let value = text.input.to_static().unwrap();
        let expr = quote_spanned! { text.input.span() => dioxus_core::static_text!(#value) };
        TypedView {
            expr: expr.clone(),
            child_expr: expr,
            ty: quote! { () },
        }
    }

    fn dynamic_node(&mut self, tokens: TokenStream2) -> TypedView {
        let id = self.dynamic_node_count;
        self.dynamic_node_count += 1;
        self.hot_reload_dynamic_nodes
            .push(quote! { dioxus_core::internal::HotReloadDynamicNode::Dynamic(#id) });
        TypedView {
            expr: quote! { dioxus_core::internal::node_dyn::<_, ()>(#tokens) },
            child_expr: tokens,
            ty: quote! { dioxus_core::internal::DynamicNodeView<dioxus_core::DynamicNode> },
        }
    }

    fn dynamic_attr(&mut self, attr: &Attribute) -> TypedAttr {
        let id = self.dynamic_attr_count;
        self.dynamic_attr_count += 1;
        self.hot_reload_dynamic_attrs
            .push(quote! { dioxus_core::internal::HotReloadDynamicAttribute::Dynamic(#id) });

        if let AttributeValue::AttrLiteral(HotLiteral::Fmted(lit)) = &attr.value {
            self.allocate_formatted(lit);
        }

        let attrs = attr.rendered_as_dynamic_attr();
        TypedAttr {
            append: AttrAppend::Direct(quote! { dioxus_core::internal::attrs_dyn(#attrs) }),
            ty: quote! { dioxus_core::view::DynAttrs },
        }
    }

    fn dynamic_builder_attr(&mut self, attr: &Attribute, method: Ident) -> TypedAttr {
        let id = self.dynamic_attr_count;
        self.dynamic_attr_count += 1;
        self.hot_reload_dynamic_attrs
            .push(quote! { dioxus_core::internal::HotReloadDynamicAttribute::Dynamic(#id) });

        if let AttributeValue::AttrLiteral(HotLiteral::Fmted(lit)) = &attr.value {
            self.allocate_formatted(lit);
        }

        let attr_value = &attr.value;
        let value = quote! { #attr_value };
        // Inline event closures need the explicit-closure builder method so the closure
        // parameter type can be inferred without an annotation.
        let method = if attr.name.is_likely_event() {
            event_handler_method(&method, &value)
        } else {
            method
        };
        TypedAttr {
            append: AttrAppend::Method { method, value },
            ty: quote! { dioxus_core::view::DynAttrs },
        }
    }

    fn component_literal_ids(&mut self, component: &Component, implicit_key: bool) -> Vec<usize> {
        let mut literal_ids = Vec::with_capacity(component.literal_component_property_count());

        for property in &component.fields {
            let AttributeValue::AttrLiteral(literal) = &property.value else {
                continue;
            };

            let hot_literal = match literal {
                HotLiteral::Fmted(fmted) => {
                    let fmted = self.allocate_formatted(fmted);
                    if property.name.is_likely_key() {
                        if implicit_key {
                            self.hot_reload_key = Some(fmted.clone());
                        }
                        continue;
                    }
                    quote! { dioxus_core::internal::HotReloadLiteral::Fmted(#fmted) }
                }
                HotLiteral::Float(value) => {
                    if property.name.is_likely_key() {
                        continue;
                    }
                    quote! { dioxus_core::internal::HotReloadLiteral::Float(#value as _) }
                }
                HotLiteral::Int(value) => {
                    if property.name.is_likely_key() {
                        continue;
                    }
                    quote! { dioxus_core::internal::HotReloadLiteral::Int(#value as _) }
                }
                HotLiteral::Bool(value) => {
                    if property.name.is_likely_key() {
                        continue;
                    }
                    quote! { dioxus_core::internal::HotReloadLiteral::Bool(#value) }
                }
            };

            let id = self.component_value_tokens.len();
            self.component_value_tokens.push(hot_literal);
            literal_ids.push(id);
        }

        literal_ids
    }

    fn allocate_formatted(&mut self, formatted: &HotReloadFormattedSegment) -> TokenStream2 {
        let mut dynamic_ids = Vec::with_capacity(formatted.formatted_segment_count());
        for segment in &formatted.segments {
            if let Segment::Formatted(segment) = segment {
                let id = self.dynamic_text_tokens.len();
                dynamic_ids.push(id);
                self.dynamic_text_tokens
                    .push(quote! { #segment.to_string() });
            }
        }
        formatted.quote_with_dynamic_ids(&dynamic_ids)
    }

    fn static_attr(
        &mut self,
        span: proc_macro2::Span,
        name: TokenStream2,
        value: TokenStream2,
        namespace: TokenStream2,
    ) -> TypedAttr {
        let marker = self.next_ident("__DioxusAttr");
        self.definitions.push(quote_spanned! { span =>
            struct #marker;
            impl dioxus_core::view::AttributeDescriptor for #marker {
                const NAME: &'static str = #name;
                const NAMESPACE: dioxus_core::TemplateRawAttrNamespace = #namespace;
            }
            impl dioxus_core::view::StaticAttributeValue for #marker {
                const VALUE: &'static str = #value;
            }
        });
        TypedAttr {
            append: AttrAppend::Direct(
                quote_spanned! { span => dioxus_core::view::attr::<#marker>() },
            ),
            ty: quote! { () },
        }
    }

    fn static_builder_attr(
        &mut self,
        span: proc_macro2::Span,
        value: TokenStream2,
        method: Ident,
    ) -> TypedAttr {
        TypedAttr {
            append: AttrAppend::Method {
                method,
                value: quote_spanned! { span => dioxus_core::static_value!(#value) },
            },
            ty: quote! { () },
        }
    }

    fn element_tag(&mut self, element: &Element) -> TypedTag {
        match &element.name {
            ElementName::Ident(tag) => TypedTag {
                expr: quote_spanned! { element.name.span() => #tag() },
                ty: quote! { () },
            },
            ElementName::Custom(_) => {
                let tag = self.define_tag(element);
                TypedTag {
                    expr: quote! { dioxus_core::view::el::<#tag>() },
                    ty: quote! { #tag },
                }
            }
        }
    }

    fn define_tag(&mut self, element: &Element) -> Ident {
        let marker = self.next_ident("__DioxusTag");
        let tag = element.name.tag_name();
        self.definitions
            .push(quote_spanned! { element.name.span() =>
                struct #marker;
                impl dioxus_core::view::TagName for #marker {
                    const NAME: &'static str = #tag;
                }
            });
        marker
    }

    fn next_ident(&mut self, prefix: &str) -> Ident {
        let index = self.next_marker;
        self.next_marker += 1;
        format_ident!("{prefix}{index}")
    }

    fn tuple_or_unit(views: Vec<TypedView>) -> TypedView {
        match views.len() {
            0 => TypedView {
                expr: quote! { () },
                child_expr: quote! { () },
                ty: quote! { () },
            },
            1 => views.into_iter().next().unwrap(),
            _ => {
                let exprs = views
                    .iter()
                    .map(|view| view.expr.clone())
                    .collect::<Vec<_>>();
                let tys = views.iter().map(|view| view.ty.clone());
                TypedView {
                    expr: quote! { (#(#exprs),*) },
                    child_expr: quote! { (#(#exprs),*) },
                    ty: quote! { (#(#tys),*) },
                }
            }
        }
    }

    fn fold_builder_tuple_type(types: impl IntoIterator<Item = TokenStream2>) -> TokenStream2 {
        let mut ty = quote! { () };
        for next in types {
            ty = quote! { (#ty, #next) };
        }
        ty
    }
}

impl Element {
    fn typed_builder_attribute(&self, attr: &Attribute, builder: &mut ViewBuilder) -> TypedAttr {
        if matches!(self.name, ElementName::Ident(_))
            && let AttributeName::BuiltIn(method) = &attr.name
            && !attr.name.is_likely_key()
        {
            if attr.name.is_likely_event() {
                return builder.dynamic_builder_attr(attr, method.clone());
            }

            if let Some((_, value)) = attr.as_static_str_literal() {
                let value = value.to_static().unwrap();
                return builder.static_builder_attr(attr.span(), quote! { #value }, method.clone());
            }

            return builder.dynamic_builder_attr(attr, method.clone());
        }

        let Some((name, value)) = attr.as_static_str_literal() else {
            return builder.dynamic_attr(attr);
        };

        let namespace = self.builder_attribute_namespace_tokens(name);
        let name = self.builder_attribute_name_tokens(name);
        let value = value.to_static().unwrap();
        builder.static_attr(attr.span(), name, quote! { #value }, namespace)
    }

    fn builder_attribute_namespace_tokens(&self, name: &AttributeName) -> TokenStream2 {
        match name.resolved(&self.name).namespace {
            Some(namespace) => quote! { Some(#namespace) },
            None => quote!(None::<&'static str>),
        }
    }

    fn builder_attribute_name_tokens(&self, name: &AttributeName) -> TokenStream2 {
        let name = name.resolved(&self.name).name;
        quote! { #name }
    }
}
