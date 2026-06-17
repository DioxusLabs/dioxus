//! Lower parsed RSX bodies into typed view builders.

use self::location::DynIdx;
use crate::*;
use dioxus_core_template::{TemplateRawOp, TemplateStorageEstimate};
use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use proc_macro2_diagnostics::SpanDiagnosticExt;
use quote::{ToTokens, TokenStreamExt, format_ident, quote, quote_spanned};
use syn::parse_quote;

const ROOT_TUPLE_VIEW_LIMIT: usize = 64;
const MAX_SYNTHETIC_CHUNKS_PER_PARENT: usize = 24;

/// A set of nodes in a template position
///
/// this could be:
/// - The root of a callbody
/// - The children of a component
/// - The children of a for loop
/// - The children of an if chain
///
/// The TemplateBody when needs to be parsed into a surrounding `Body` to be correctly re-indexed
/// By default every body has a `0` default index
#[derive(PartialEq, Eq, Clone, Debug)]
pub struct TemplateBody {
    pub roots: Vec<BodyNode>,
    pub template_idx: DynIdx,
    pub diagnostics: Diagnostics,
}

impl Parse for TemplateBody {
    /// Parse the nodes of the callbody as `Body`.
    fn parse(input: ParseStream) -> Result<Self> {
        let children = RsxBlock::parse_children(input)?;
        let mut myself = Self::new(children.children);
        myself
            .diagnostics
            .extend(children.diagnostics.into_diagnostics());

        Ok(myself)
    }
}

/// Our ToTokens impl here just defers to rendering a template out like any other `Body`.
/// This is because the parsing phase filled in all the additional metadata we need
impl ToTokens for TemplateBody {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        // First normalize the template body for rendering
        let node = self.normalized();

        // If we have an implicit key, then we need to write its tokens
        let key_tokens = match node.implicit_key() {
            Some(tok) => quote! { Some( #tok.to_string() ) },
            None => quote! { None },
        };

        let key_warnings = self.check_for_duplicate_keys();

        let view = node.view_builder_pieces();
        let view_definitions = view.definitions();
        let view_expr = view.view_expr();
        let dynamic_text = view.dynamic_text_tokens().iter();

        let diagnostics = &node.diagnostics;
        let index = node.template_idx.get();
        let hot_reload_mapping = view.hot_reload_template_tokens(quote! { __template });

        tokens.append_all(quote! {
            dioxus_core::Element::Ok({
                #diagnostics

                #key_warnings

                #(#view_definitions)*

                #[cfg(debug_assertions)]
                let mut __dynamic_literal_pool = dioxus_core::internal::DynamicLiteralPool::new(
                    vec![ #( #dynamic_text.to_string() ),* ],
                );

                #[cfg(debug_assertions)]
                let __hot_reload_template_read = {
                    use dioxus_signals::ReadableExt;

                    static __NORMALIZED_FILE: &'static str = {
                        const PATH: &str = dioxus_core::const_format::str_replace!(file!(), "\\\\", "/");
                        dioxus_core::const_format::str_replace!(PATH, '\\', "/")
                    };

                    // The key is important here - we're creating a new GlobalSignal each call to this
                    // But the key is what's keeping it stable
                    static __TEMPLATE: dioxus_signals::GlobalSignal<Option<dioxus_core::internal::HotReloadedTemplate>> = dioxus_signals::GlobalSignal::with_location(
                        || None::<dioxus_core::internal::HotReloadedTemplate>,
                        __NORMALIZED_FILE,
                        line!(),
                        column!(),
                        #index
                    );

                    dioxus_core::Runtime::try_current().map(|_| __TEMPLATE.read())
                };

                // The key needs to be created before the dynamic nodes as it might depend on a borrowed value which gets moved into the dynamic nodes
                let __key = #key_tokens;

                // NOTE: Allocating a temporary is important to make reads within rsx drop before the value is returned
                #[allow(clippy::let_and_return)]
                let __vnodes = {
                    use dioxus_core::view::View as _;
                    #view_expr.key(__key).into_vnode()
                };

                #[cfg(debug_assertions)]
                {
                    let __template = __vnodes.template;
                    let __original_template = #hot_reload_mapping;
                    // If the template has not been hot reloaded, we always use the original template
                    // Templates nested within macros may be merged because they have the same file-line-column-index
                    // They cannot be hot reloaded, so this prevents incorrect rendering
                    let __template_read = match __hot_reload_template_read.as_ref().map(|__template_read| __template_read.as_ref()) {
                        Some(Some(__template_read)) => &__template_read,
                        _ => &__original_template,
                    };

                    let mut __dynamic_value_pool = dioxus_core::internal::DynamicValuePool::from_vnode(
                        &__vnodes,
                        __dynamic_literal_pool
                    );
                    __dynamic_value_pool.render_with(__template_read)
                }
                #[cfg(not(debug_assertions))]
                {
                    __vnodes
                }
            })
        });
    }
}

pub(crate) struct ViewBuilderPieces {
    definitions: Vec<TokenStream2>,
    view: TokenStream2,
    dynamic_text_tokens: Vec<TokenStream2>,
    component_value_tokens: Vec<TokenStream2>,
    hot_reload_dynamic_nodes: Vec<TokenStream2>,
    hot_reload_dynamic_attrs: Vec<TokenStream2>,
    hot_reload_dynamic_slots: Vec<TokenStream2>,
    hot_reload_key: Option<TokenStream2>,
}

impl ViewBuilderPieces {
    fn from_body(body: &TemplateBody) -> Self {
        let mut builder = ViewBuilder::new();
        let view = builder.visit_roots(&body.roots);
        builder.finish(view)
    }

    fn from_element(element: &Element) -> Self {
        let mut builder = ViewBuilder::new();
        let view = builder.visit_element(element, true).expr;
        builder.finish(view)
    }

    pub(crate) fn definitions(&self) -> impl Iterator<Item = &TokenStream2> {
        self.definitions.iter()
    }

    pub(crate) fn view_expr(&self) -> &TokenStream2 {
        &self.view
    }

    fn dynamic_text_tokens(&self) -> &[TokenStream2] {
        &self.dynamic_text_tokens
    }

    fn hot_reload_template_tokens(&self, template: TokenStream2) -> TokenStream2 {
        let key = self
            .hot_reload_key
            .as_ref()
            .map(|key| quote! { Some(#key) })
            .unwrap_or_else(|| quote! { None });
        let dynamic_nodes = self.hot_reload_dynamic_nodes.iter();
        let dyn_attrs = self.hot_reload_dynamic_attrs.iter();
        let component_values = self.component_value_tokens.iter();
        let dynamic_slots = self.hot_reload_dynamic_slots.iter();

        quote! {
            dioxus_core::internal::HotReloadedTemplate::from_template(
                #key,
                vec![ #( #dynamic_nodes ),* ],
                vec![ #( #dyn_attrs ),* ],
                vec![ #( #component_values ),* ],
                #template,
                vec![ #( #dynamic_slots ),* ],
            )
        }
    }
}

struct ViewExpr {
    expr: TokenStream2,
    child_arg: Option<TokenStream2>,
}

impl ViewExpr {
    fn into_child_arg(self) -> TokenStream2 {
        self.child_arg.unwrap_or(self.expr)
    }
}

#[derive(Clone, Copy)]
enum SiblingContext {
    Roots,
    ElementChildren,
}

struct ViewBuilder {
    definitions: Vec<TokenStream2>,
    dynamic_node_count: usize,
    dynamic_attr_count: usize,
    dynamic_text_tokens: Vec<TokenStream2>,
    component_value_tokens: Vec<TokenStream2>,
    hot_reload_dynamic_nodes: Vec<TokenStream2>,
    hot_reload_dynamic_attrs: Vec<TokenStream2>,
    hot_reload_dynamic_slots: Vec<TokenStream2>,
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
            hot_reload_dynamic_slots: Vec::new(),
            hot_reload_key: None,
            next_marker: 0,
        }
    }

    fn finish(self, view: TokenStream2) -> ViewBuilderPieces {
        ViewBuilderPieces {
            definitions: self.definitions,
            view,
            dynamic_text_tokens: self.dynamic_text_tokens,
            component_value_tokens: self.component_value_tokens,
            hot_reload_dynamic_nodes: self.hot_reload_dynamic_nodes,
            hot_reload_dynamic_attrs: self.hot_reload_dynamic_attrs,
            hot_reload_dynamic_slots: self.hot_reload_dynamic_slots,
            hot_reload_key: self.hot_reload_key,
        }
    }

    fn visit_roots(&mut self, nodes: &[BodyNode]) -> TokenStream2 {
        self.visit_roots_with_implicit_key(nodes, true)
    }

    fn visit_roots_with_implicit_key(
        &mut self,
        nodes: &[BodyNode],
        allow_implicit_key: bool,
    ) -> TokenStream2 {
        let roots = self
            .visit_sibling_nodes(nodes, allow_implicit_key, SiblingContext::Roots)
            .into_iter()
            .map(|view| view.expr);

        quote! { (#(#roots),*) }
    }

    fn visit_sibling_nodes(
        &mut self,
        nodes: &[BodyNode],
        allow_implicit_key: bool,
        context: SiblingContext,
    ) -> Vec<ViewExpr> {
        if self.should_chunk_siblings(nodes, context) {
            if matches!(context, SiblingContext::Roots) {
                return vec![self.synthetic_dynamic_chunk(nodes)];
            }

            return self
                .synthetic_chunk_ranges(nodes, context)
                .into_iter()
                .map(|range| self.synthetic_dynamic_chunk(&nodes[range]))
                .collect();
        }

        nodes
            .iter()
            .enumerate()
            .map(|(index, node)| self.visit_node(node, allow_implicit_key && index == 0))
            .collect()
    }

    fn visit_node(&mut self, node: &BodyNode, implicit_key: bool) -> ViewExpr {
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

    fn visit_element(&mut self, element: &Element, implicit_key: bool) -> ViewExpr {
        let tag = self.element_tag(element);
        let mut attrs = TokenStream2::new();
        for attr in &element.merged_attributes {
            attrs.extend(element.typed_builder_attribute(attr, self));
        }

        if let Some(AttributeValue::AttrLiteral(HotLiteral::Fmted(key))) = element.key() {
            let key = self.allocate_formatted(key);
            if implicit_key {
                self.hot_reload_key = Some(key);
            }
        }

        let mut children = TokenStream2::new();
        for child in
            self.visit_sibling_nodes(&element.children, false, SiblingContext::ElementChildren)
        {
            let child = child.into_child_arg();
            children.extend(quote! { .child(#child) });
        }

        ViewExpr {
            expr: quote! { #tag #attrs #children },
            child_arg: None,
        }
    }

    fn static_text(&mut self, text: &TextNode) -> ViewExpr {
        let value = text.input.to_static().unwrap();
        let expr = quote_spanned! { text.input.span() => dioxus_core::static_text!(#value) };
        ViewExpr {
            expr,
            child_arg: None,
        }
    }

    fn dynamic_node(&mut self, tokens: TokenStream2) -> ViewExpr {
        let id = self.dynamic_node_count;
        self.dynamic_node_count += 1;
        self.hot_reload_dynamic_nodes
            .push(quote! { dioxus_core::internal::HotReloadDynamicNode::Dynamic(#id) });
        self.hot_reload_dynamic_slots
            .push(quote! { dioxus_core::internal::HotReloadDynamicSlot::Node(#id) });
        ViewExpr {
            expr: quote! { dioxus_core::internal::dynamic_node_builder::<_, ()>(#tokens) },
            child_arg: Some(tokens),
        }
    }

    fn dynamic_attr(&mut self, attr: &Attribute) -> TokenStream2 {
        self.track_dynamic_attr(attr);
        let attrs = attr.rendered_as_dynamic_attr();
        quote! { .attribute(dioxus_core::internal::dynamic_attributes_builder(#attrs)) }
    }

    fn dynamic_builder_attr(&mut self, attr: &Attribute, method: Ident) -> TokenStream2 {
        self.track_dynamic_attr(attr);
        let attr_value = &attr.value;
        let value = quote! { #attr_value };
        let method = if attr.name.is_likely_event() {
            event_handler_method(&method, &value)
        } else {
            method
        };
        quote! { .#method(#value) }
    }

    fn track_dynamic_attr(&mut self, attr: &Attribute) {
        let id = self.dynamic_attr_count;
        self.dynamic_attr_count += 1;
        self.hot_reload_dynamic_attrs
            .push(quote! { dioxus_core::internal::HotReloadDynamicAttribute::Dynamic(#id) });
        self.hot_reload_dynamic_slots
            .push(quote! { dioxus_core::internal::HotReloadDynamicSlot::Attribute(#id) });
        if let AttributeValue::AttrLiteral(HotLiteral::Fmted(lit)) = &attr.value {
            self.allocate_formatted(lit);
        }
    }

    fn component_literal_ids(&mut self, component: &Component, implicit_key: bool) -> Vec<usize> {
        let mut literal_ids = Vec::with_capacity(component.literal_component_property_count());

        for property in &component.fields {
            let AttributeValue::AttrLiteral(literal) = &property.value else {
                continue;
            };

            if property.name.is_likely_key() {
                if let HotLiteral::Fmted(fmted) = literal {
                    let fmted = self.allocate_formatted(fmted);
                    if implicit_key {
                        self.hot_reload_key = Some(fmted);
                    }
                }
                continue;
            }

            let hot_literal = match literal {
                HotLiteral::Fmted(fmted) => {
                    let fmted = self.allocate_formatted(fmted);
                    quote! { dioxus_core::internal::HotReloadLiteral::Fmted(#fmted) }
                }
                HotLiteral::Float(value) => {
                    quote! { dioxus_core::internal::HotReloadLiteral::Float(#value as _) }
                }
                HotLiteral::Int(value) => {
                    quote! { dioxus_core::internal::HotReloadLiteral::Int(#value as _) }
                }
                HotLiteral::Bool(value) => {
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
    ) -> TokenStream2 {
        let marker = self.next_ident("__DioxusAttr");
        self.definitions.push(quote_spanned! { span =>
            struct #marker;
            impl dioxus_core::view::AttributeDescriptor for #marker {
                const NAME: &'static str = #name;
                const NAMESPACE: Option<&'static str> = #namespace;
            }
            impl dioxus_core::view::StaticAttributeValue for #marker {
                const VALUE: &'static str = #value;
            }
        });
        let attr = quote_spanned! { span => dioxus_core::view::static_attribute::<#marker>() };
        quote! { .attribute(#attr) }
    }

    fn static_builder_attr(
        &mut self,
        span: proc_macro2::Span,
        value: TokenStream2,
        method: Ident,
    ) -> TokenStream2 {
        let value = quote_spanned! { span => dioxus_core::static_attribute_value!(#value) };
        quote! { .#method(#value) }
    }

    fn element_tag(&mut self, element: &Element) -> TokenStream2 {
        match &element.name {
            ElementName::Ident(tag) => quote_spanned! { element.name.span() => #tag() },
            ElementName::Custom(_) => {
                let tag = self.define_tag(element);
                quote! { dioxus_core::view::element_builder::<#tag>() }
            }
        }
    }

    fn define_tag(&mut self, element: &Element) -> Ident {
        let marker = self.next_ident("__DioxusTag");
        let tag = element.name.tag_name();
        self.definitions
            .push(quote_spanned! { element.name.span() =>
                struct #marker;
                impl dioxus_core::view::ElementTag for #marker {
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

    fn synthetic_dynamic_chunk(&mut self, nodes: &[BodyNode]) -> ViewExpr {
        let mut chunk_builder = ViewBuilder::new();
        let roots = nodes
            .iter()
            .map(|node| chunk_builder.visit_node(node, false))
            .map(|view| {
                let expr = view.expr;
                quote! { #expr.into_vnode() }
            })
            .collect::<Vec<_>>();
        let chunk = chunk_builder.finish(quote! { () });
        let definitions = chunk.definitions();
        let tokens = quote! {{
            use dioxus_core::view::View as _;
            #(#definitions)*
            dioxus_core::DynamicNode::Fragment(vec![#(#roots),*])
        }};
        self.dynamic_node(tokens)
    }

    fn should_chunk_siblings(&self, nodes: &[BodyNode], context: SiblingContext) -> bool {
        if nodes.len() <= 1 {
            return false;
        }

        matches!(context, SiblingContext::Roots) && nodes.len() > ROOT_TUPLE_VIEW_LIMIT
            || self
                .estimate_siblings_unwrapped(nodes)
                .exceeds_storage_limits()
    }

    fn synthetic_chunk_ranges(
        &self,
        nodes: &[BodyNode],
        context: SiblingContext,
    ) -> Vec<std::ops::Range<usize>> {
        let estimate = self.estimate_siblings_unwrapped(nodes);
        let mut chunks = estimate.max_required_chunks();

        if matches!(context, SiblingContext::Roots) {
            chunks = chunks.max(nodes.len().div_ceil(ROOT_TUPLE_VIEW_LIMIT));
        }

        chunks = chunks.clamp(2, MAX_SYNTHETIC_CHUNKS_PER_PARENT.min(nodes.len()));
        let chunk_len = nodes.len().div_ceil(chunks);

        (0..nodes.len())
            .step_by(chunk_len)
            .map(|start| start..(start + chunk_len).min(nodes.len()))
            .collect()
    }

    fn estimate_siblings_unwrapped(&self, nodes: &[BodyNode]) -> TemplateStorageEstimate {
        let mut raw = Vec::new();
        self.push_sibling_raw_ops(nodes, &mut raw);
        TemplateStorageEstimate::from_raw_ops(&raw)
    }

    fn push_sibling_raw_ops(&self, nodes: &[BodyNode], raw: &mut Vec<TemplateRawOp>) {
        for node in nodes {
            self.push_node_raw_ops(node, raw);
        }
    }

    fn push_node_raw_ops(&self, node: &BodyNode, raw: &mut Vec<TemplateRawOp>) {
        match node {
            BodyNode::Element(element) => self.push_element_raw_ops(element, raw),
            BodyNode::Text(text) if text.is_static() => {
                raw.push(TemplateRawOp::StaticText { value: "" })
            }
            BodyNode::Text(_)
            | BodyNode::RawExpr(_)
            | BodyNode::Component(_)
            | BodyNode::ForLoop(_)
            | BodyNode::IfChain(_) => raw.push(TemplateRawOp::DynamicNode),
        }
    }

    fn push_element_raw_ops(&self, element: &Element, raw: &mut Vec<TemplateRawOp>) {
        raw.push(TemplateRawOp::OpenElement {
            tag: "",
            namespace: None,
        });

        for attr in &element.merged_attributes {
            self.push_attribute_raw_op(element, attr, raw);
        }

        if self.should_chunk_siblings(&element.children, SiblingContext::ElementChildren) {
            for _ in self.synthetic_chunk_ranges(&element.children, SiblingContext::ElementChildren)
            {
                raw.push(TemplateRawOp::DynamicNode);
            }
        } else {
            self.push_sibling_raw_ops(&element.children, raw);
        }

        raw.push(TemplateRawOp::CloseElement);
    }

    fn push_attribute_raw_op(
        &self,
        element: &Element,
        attr: &Attribute,
        raw: &mut Vec<TemplateRawOp>,
    ) {
        if attr.as_static_str_literal().is_some() && !attr.name.is_likely_event() {
            let namespace = attr.name.resolved(&element.name).namespace.map(|_| "");
            raw.push(TemplateRawOp::StaticAttr {
                name: "",
                value: "",
                namespace,
            });
        } else {
            raw.push(TemplateRawOp::DynamicAttr);
        }
    }
}

impl Element {
    pub(crate) fn view_builder_pieces(&self) -> ViewBuilderPieces {
        ViewBuilderPieces::from_element(self)
    }

    fn typed_builder_attribute(&self, attr: &Attribute, builder: &mut ViewBuilder) -> TokenStream2 {
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

impl TemplateBody {
    pub(crate) fn view_builder_pieces(&self) -> ViewBuilderPieces {
        ViewBuilderPieces::from_body(self)
    }

    /// Create a new TemplateBody from a set of nodes
    ///
    /// This will fill in all the necessary path information for the nodes in the template and will
    /// overwrite data like dynamic indexes.
    pub fn new(nodes: Vec<BodyNode>) -> Self {
        let mut body = Self {
            roots: vec![],
            template_idx: DynIdx::default(),
            diagnostics: Diagnostics::new(),
        };

        // Save the roots without mutating the parsed tree; template lowering derives dynamic
        // positions from the raw op tape.
        body.roots = nodes;

        // Finally, validate the key
        body.validate_key();

        body
    }

    /// Normalize the Template body for rendering. If the body is completely empty, insert a placeholder node
    pub fn normalized(&self) -> Self {
        // If the nodes are completely empty, insert a placeholder node
        // Core expects at least one node in the template to make it easier to replace
        if self.is_empty() {
            // Create an empty template body with a placeholder and diagnostics + the template index from the original
            let empty = Self::new(vec![BodyNode::RawExpr(parse_quote! {()})]);
            let default = Self {
                diagnostics: self.diagnostics.clone(),
                template_idx: self.template_idx.clone(),
                ..empty
            };
            return default;
        }
        self.clone()
    }

    pub fn is_empty(&self) -> bool {
        self.roots.is_empty()
    }

    pub fn implicit_key(&self) -> Option<&AttributeValue> {
        self.roots.first().and_then(BodyNode::key)
    }

    /// Ensure only one key and that the key is not a static str
    ///
    /// todo: we want to allow arbitrary exprs for keys provided they impl hash / eq
    fn validate_key(&mut self) {
        let key = self.implicit_key();

        if let Some(attr) = key {
            let diagnostic = match &attr {
                AttributeValue::AttrLiteral(ifmt) => {
                    if ifmt.is_static() {
                        ifmt.span().error("Key must not be a static string. Make sure to use a formatted string like `key: \"{value}\"")
                    } else {
                        return;
                    }
                }
                _ => attr
                    .span()
                    .error("Key must be in the form of a formatted string like `key: \"{value}\""),
            };

            self.diagnostics.push(diagnostic);
        }
    }

    fn check_for_duplicate_keys(&self) -> TokenStream2 {
        let mut warnings = TokenStream2::new();

        // Make sure there are not multiple keys or keys on nodes other than the first in the block
        for root in self.roots.iter().skip(1) {
            if let Some(key) = root.key() {
                warnings.extend(new_diagnostics::warning_diagnostic(
                    key.span(),
                    "Keys are only allowed on the first node in the block.",
                ));
            }
        }

        warnings
    }

    /// Get the span of the first root of this template
    pub(crate) fn first_root_span(&self) -> Span {
        match self.roots.first() {
            Some(root) => root.span(),
            _ => Span::call_site(),
        }
    }
}
