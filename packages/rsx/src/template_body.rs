//! Lower parsed RSX bodies into static templates plus dynamic node and attribute values.

use self::location::DynIdx;
use crate::stats::{TemplateStatsBuilder, TemplateStorageStats};
use crate::*;
use dioxus_core_template::{TEMPLATE_SLOT_PATH_MAX_PATH_BITS, TEMPLATE_STORAGE_MAX_CAP};
use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use proc_macro2_diagnostics::SpanDiagnosticExt;
use quote::{ToTokens, TokenStreamExt, format_ident, quote, quote_spanned};
use std::collections::HashMap;
use syn::parse_quote;

/// `View`/`ViewTemplate` are only implemented for tuples up to this arity, so sibling lists wider
/// than this are emitted as several tuples joined structurally (see `group_sibling_views`).
const MAX_TUPLE_VIEW_ARITY: usize = 128;

/// Join per-sibling typed views into one tuple view.
///
/// A sibling list wider than [`MAX_TUPLE_VIEW_ARITY`] cannot be a single tuple, so it is split into
/// several, which are in turn grouped until one tuple remains. The split is transparent to the
/// lowered template: each tuple lowers to a `Sequence`, and nested sequences flatten to exactly the
/// same ops and dynamic-slot order as one flat list.
fn group_sibling_views(mut views: Vec<TokenStream2>) -> TokenStream2 {
    if views.is_empty() {
        return quote! { () };
    }
    loop {
        views = views
            .chunks(MAX_TUPLE_VIEW_ARITY)
            .map(|chunk| {
                let chunk = chunk.iter();
                quote! { (#(#chunk,)*) }
            })
            .collect();
        if views.len() == 1 {
            return views.pop().unwrap();
        }
    }
}

/// Drives a [`TemplateStatsBuilder`] from a canonical fill-order walk so the predicted op/string/
/// anchor capacities match the ops the typed view builder emits. The single authoritative
/// traversal lives in [`crate::visit_roots`]; this is its stats consumer.
#[derive(Default)]
struct StatsCollector {
    stats: TemplateStatsBuilder,
    following_static_at_parent: bool,
}

impl<'a> FillOrderVisitor<'a> for StatsCollector {
    fn visit_siblings(&mut self, nodes: &'a [BodyNode]) -> Option<()> {
        for (index, node) in nodes.iter().enumerate() {
            let previous = self.following_static_at_parent;
            self.following_static_at_parent = siblings_have_static_node(nodes, index + 1);
            let result = FillOrderVisitor::visit_node(self, node);
            self.following_static_at_parent = previous;
            result?;
        }
        Some(())
    }

    fn open_element(&mut self, _element: &'a Element) -> Option<()> {
        self.stats.open_element(None);
        Some(())
    }

    fn close_element(&mut self, _element: &'a Element) -> Option<()> {
        self.stats.close_element();
        Some(())
    }

    fn static_attribute(&mut self, _element: &'a Element, _attr: &'a Attribute) -> Option<()> {
        self.stats.static_attr(None);
        Some(())
    }

    fn dynamic_attribute(&mut self, _element: &'a Element, _attr: &'a Attribute) -> Option<()> {
        self.stats.dynamic_attr();
        Some(())
    }

    fn static_text(&mut self, _text: &'a TextNode) -> Option<()> {
        self.stats.static_text();
        Some(())
    }

    fn dynamic_node(&mut self, _node: &'a BodyNode) -> Option<()> {
        self.stats.dynamic_node(self.following_static_at_parent);
        Some(())
    }
}

/// Predicted storage stats for a sibling list, derived from the canonical fill-order walk.
fn sibling_storage_stats(nodes: &[BodyNode]) -> TemplateStorageStats {
    let mut collector = StatsCollector::default();
    visit_roots(&mut collector, nodes);
    collector.stats.finish()
}

/// Predicted storage stats for a single element subtree.
fn element_storage_stats(element: &Element) -> TemplateStorageStats {
    let mut collector = StatsCollector::default();
    visit_element(&mut collector, element);
    collector.stats.finish()
}

/// A set of nodes in a template position
///
/// this could be:
/// - The root of a callbody
/// - The children of a component
/// - The children of a for loop
/// - The children of an if chain
///
/// `CallBody` assigns each `TemplateBody` a hot-reload template index after the whole `rsx!` body
/// has been parsed.
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

        let key_warnings = self.check_for_duplicate_keys();

        // Build the typed view once: the release tree, the capacities, and (in debug) the
        // hot-reload tables all come from this one traversal.
        let pieces = ViewBuilderPieces::from_body(&node);
        let view_definitions = pieces.definitions.iter();
        let raw_view_expr = &pieces.view;
        // Dynamic node values are pushed into `__dynamic` (in template order) before the builder
        // chain so that any borrows they take are released before the chain moves captured values
        // into event-handler closures. This intentionally matches the 0.6 dynamic-node-before-
        // attribute evaluation order instead of the more straightforward typed-builder evaluation
        // order. It also keeps node values out of the typed view, which then only carries dynamic
        // attributes and is zero-sized for most bodies. The key is bound first because it may
        // borrow a value that one of those dynamic nodes moves.
        let node_pushes = &pieces.node_pushes;
        let node_count = pieces.dynamic_node_count;
        let attr_count = pieces.dynamic_attr_count;
        let implicit_key = node.implicit_key();
        // Only bodies with formatted text or component literals read from the hot-reload literal
        // pool while the view evaluates; every other body defers the hot-reload read to the site.
        let has_literal_pool =
            !pieces.dynamic_text_tokens.is_empty() || !pieces.component_value_tokens.is_empty();
        let is_static = node_pushes.is_empty()
            && attr_count == 0
            && implicit_key.is_none()
            && !has_literal_pool;
        let dynamic_values = quote! { dioxus_core::view::dynamic_values(#node_count, #attr_count) };
        let view_expr = match implicit_key {
            Some(key) => quote! {{
                use dioxus_core::view::ViewKeyExt as _;
                // The key needs to be created before the dynamic nodes as it might depend on a borrowed value which gets moved into the dynamic nodes.
                let __key = Some(#key.to_string());
                #(#node_pushes)*
                #raw_view_expr.key(__key)
            }},
            None => quote! {{
                #(#node_pushes)*
                #raw_view_expr
            }},
        };
        let dynamic_text = pieces.dynamic_text_tokens.iter();

        let template_stats = pieces.template_stats;
        let template_ops_cap = template_stats.ops;
        let template_string_cap = template_stats.strings;
        let template_dynamic_cap = template_stats.anchors;
        let release_vnode = |dynamic: TokenStream2| {
            quote! {
                dioxus_core::view::into_vnode_with_capacity::<
                    #template_ops_cap,
                    #template_string_cap,
                    #template_dynamic_cap,
                    _,
                >(__view, #dynamic)
            }
        };

        let index = node.template_idx.get();
        // The site is keyed by its location and template index; the hot-reload slot lives in the
        // runtime's global-signal context and the lowered template in a per-tree cache, so a site
        // emits no `static` of its own. Only referenced inside `#[cfg(debug_assertions)]` blocks.
        let site = quote! { file!(), line!(), column!(), #index };
        let hot_reload_meta = pieces.hot_reload_meta_tokens();

        // In release the optimized template is the const `&'static Template` built through the
        // type system. In debug builds that per-site const evaluation dominates compile time, so
        // the identical template is lowered once at runtime and cached per tree.
        let vnode = if is_static {
            // Nothing runs between building the zero-sized view and handing its tree to the site,
            // so the body needs no `DynamicValues` binding and no per-render hot-reload state.
            let release = release_vnode(dynamic_values);
            quote! {
                let __view = #view_expr;

                #[cfg(not(debug_assertions))]
                {
                    #release
                }

                #[cfg(debug_assertions)]
                {
                    dioxus_signals::render_static_site(
                        #site,
                        dioxus_core::view::template_tree(&__view),
                    )
                }
            }
        } else {
            // Only the debug path mutates `__dynamic` after the node pushes (`push_view`).
            let dynamic_binding = if node_pushes.is_empty() {
                quote! {
                    #[cfg(debug_assertions)]
                    let mut __dynamic = #dynamic_values;
                    #[cfg(not(debug_assertions))]
                    let __dynamic = #dynamic_values;
                }
            } else {
                quote! { let mut __dynamic = #dynamic_values; }
            };
            let release = release_vnode(quote! { __dynamic });
            let debug_render = if has_literal_pool {
                quote! { __hot_reload_site.finish(__tree, __dynamic) }
            } else {
                quote! { dioxus_signals::render_site(#site, #hot_reload_meta, __tree, __dynamic) }
            };
            quote! {
                #dynamic_binding
                let __view = #view_expr;

                #[cfg(not(debug_assertions))]
                {
                    #release
                }

                #[cfg(debug_assertions)]
                {
                    let __tree = dioxus_core::view::template_tree(&__view);
                    dioxus_core::view::push_view(__view, &mut __dynamic);
                    #debug_render
                }
            }
        };

        // The site's hot-reload read and literal pool must be in scope before the view is built:
        // component literal props pull their hot-reloaded value from the site while the view
        // expression evaluates. The read guard stays alive until `finish`.
        let hot_reload_site = has_literal_pool.then(|| {
            quote! {
                #[cfg(debug_assertions)]
                let __hot_reload_site = dioxus_signals::HotReloadSite::new(
                    #site,
                    #hot_reload_meta,
                    vec![ #( #dynamic_text ),* ],
                );
            }
        });

        let diagnostics = &node.diagnostics;

        tokens.append_all(quote! {
            dioxus_core::Element::Ok({
                #diagnostics

                #key_warnings

                #(#view_definitions)*

                #hot_reload_site

                #vnode
            })
        });
    }
}

pub(crate) struct ViewBuilderPieces {
    definitions: Vec<TokenStream2>,
    view: TokenStream2,
    node_pushes: Vec<TokenStream2>,
    template_stats: TemplateStorageStats,
    dynamic_text_tokens: Vec<TokenStream2>,
    component_value_tokens: Vec<TokenStream2>,
    dynamic_node_count: usize,
    dynamic_attr_count: usize,
    hot_reload_key: Option<TokenStream2>,
}

impl ViewBuilderPieces {
    fn from_element(element: &Element) -> Self {
        let mut builder = ViewBuilder::new();
        let template_stats = element_storage_stats(element);
        let view = builder.visit_element_with_diagnostics(element, true, false);
        builder.finish(view, template_stats)
    }

    /// Walk all roots of a body into a single tuple `View` expression, carrying out the
    /// hot-reload tables and dynamic text pool gathered along the way.
    fn from_body(body: &TemplateBody) -> Self {
        let mut builder = ViewBuilder::new();
        let template_stats = sibling_storage_stats(&body.roots);
        let views = builder.visit_sibling_nodes(&body.roots, true);
        let view = group_sibling_views(views);
        builder.finish(view, template_stats)
    }

    pub(crate) fn definitions(&self) -> impl Iterator<Item = &TokenStream2> {
        self.definitions.iter()
    }

    pub(crate) fn view_expr(&self) -> &TokenStream2 {
        &self.view
    }

    /// Emit the site's original-template metadata (a `HotReloadSiteMeta` initializer) from the
    /// tables gathered while building the view. Everything in it is a literal, so the slices it
    /// borrows are promoted to `'static` and the value itself is a handful of words to build.
    ///
    /// Callers must only reference the result inside a `#[cfg(debug_assertions)]` block so release
    /// expansions contain no hot-reload tokens.
    fn hot_reload_meta_tokens(&self) -> TokenStream2 {
        let key = self
            .hot_reload_key
            .as_ref()
            .map(|key| quote! { Some(#key) })
            .unwrap_or_else(|| quote! { None });
        // Dynamic nodes/attributes are always `Dynamic(0..n)` in order, so only the
        // counts cross the expansion boundary.
        let node_count = self.dynamic_node_count;
        let attr_count = self.dynamic_attr_count;
        let component_values = self.component_value_tokens.iter();

        quote! {
            dioxus_core::internal::HotReloadSiteMeta {
                key: #key,
                dynamic_nodes: #node_count,
                dynamic_attributes: #attr_count,
                component_values: &[ #( #component_values ),* ],
            }
        }
    }
}

#[derive(Clone, Copy)]
enum SiblingContext {
    Roots,
    ElementChildren,
}

struct ViewBuilder {
    definitions: Vec<TokenStream2>,
    node_pushes: Vec<TokenStream2>,
    dynamic_node_count: usize,
    dynamic_attr_count: usize,
    dynamic_text_tokens: Vec<TokenStream2>,
    component_value_tokens: Vec<TokenStream2>,
    /// Static text and attribute-value literals, deduplicated into one per-body string table
    /// that the view refers to by index.
    static_strings: Vec<TokenStream2>,
    static_string_indices: HashMap<String, usize>,
    hot_reload_key: Option<TokenStream2>,
    next_marker: usize,
}

impl ViewBuilder {
    fn new() -> Self {
        Self {
            definitions: Vec::new(),
            node_pushes: Vec::new(),
            dynamic_node_count: 0,
            dynamic_attr_count: 0,
            dynamic_text_tokens: Vec::new(),
            component_value_tokens: Vec::new(),
            static_strings: Vec::new(),
            static_string_indices: HashMap::new(),
            hot_reload_key: None,
            next_marker: 0,
        }
    }

    fn finish(
        mut self,
        view: TokenStream2,
        template_stats: TemplateStorageStats,
    ) -> ViewBuilderPieces {
        if !self.static_strings.is_empty() {
            let strings = &self.static_strings;
            self.definitions.push(quote! {
                struct __DioxusStrings;
                impl dioxus_core::view::StaticStrings for __DioxusStrings {
                    const STRINGS: &'static [&'static str] = &[#(#strings),*];
                }
            });
        }
        ViewBuilderPieces {
            definitions: self.definitions,
            view,
            node_pushes: self.node_pushes,
            template_stats,
            dynamic_text_tokens: self.dynamic_text_tokens,
            component_value_tokens: self.component_value_tokens,
            dynamic_node_count: self.dynamic_node_count,
            dynamic_attr_count: self.dynamic_attr_count,
            hot_reload_key: self.hot_reload_key,
        }
    }

    /// Lower each sibling into its own typed view. Siblings stay one-to-one with template slots;
    /// grouping a wide list to satisfy the tuple-arity limit happens at the emit site via
    /// `group_sibling_views`, which is transparent to the lowered template.
    fn visit_sibling_nodes(
        &mut self,
        nodes: &[BodyNode],
        allow_implicit_key: bool,
    ) -> Vec<TokenStream2> {
        nodes
            .iter()
            .enumerate()
            .map(|(index, node)| self.visit_node(node, allow_implicit_key && index == 0))
            .collect()
    }

    fn visit_node(&mut self, node: &BodyNode, implicit_key: bool) -> TokenStream2 {
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
            BodyNode::SyntheticBoundary(_) => self.dynamic_node(quote! { #node }),
        }
    }

    fn visit_element(&mut self, element: &Element, implicit_key: bool) -> TokenStream2 {
        self.visit_element_with_diagnostics(element, implicit_key, true)
    }

    fn visit_element_with_diagnostics(
        &mut self,
        element: &Element,
        implicit_key: bool,
        emit_diagnostics: bool,
    ) -> TokenStream2 {
        let tag = self.element_tag(element);

        let mut attrs = TokenStream2::new();
        for attr in &element.merged_attributes {
            attrs.extend(element.typed_builder_attribute(attr, self));
        }

        // Allocate the key's formatted segments before the children's. The canonical fill order
        // (and the hot-reload `LastBuildState` pool) is attributes, key, then children, so the
        // runtime dynamic-text pool must place the key's segments ahead of the children's to keep
        // both pools in lockstep.
        if let Some(AttributeValue::AttrLiteral(HotLiteral::Fmted(key))) = element.key() {
            let key = self.allocate_formatted(key);
            if implicit_key {
                self.hot_reload_key = Some(key);
            }
        }

        let diagnostics = &element.diagnostics;
        let view = if element.children.is_empty() {
            quote! { #tag #attrs }
        } else {
            let children = group_sibling_views(self.visit_sibling_nodes(&element.children, false));
            quote! { dioxus_core::view::ElementWithChildren(#tag #attrs, #children) }
        };

        if emit_diagnostics {
            quote! {{
                #diagnostics
                #view
            }}
        } else {
            view
        }
    }

    /// The marker type for a static literal: its index into the per-body string table.
    fn static_string(&mut self, span: proc_macro2::Span, value: String) -> TokenStream2 {
        let index = match self.static_string_indices.get(value.as_str()) {
            Some(index) => *index,
            None => {
                let index = self.static_strings.len();
                self.static_strings.push(quote_spanned! { span => #value });
                self.static_string_indices.insert(value, index);
                index
            }
        };
        quote! { dioxus_core::view::Str::<__DioxusStrings, #index> }
    }

    fn static_text(&mut self, text: &TextNode) -> TokenStream2 {
        let span = text.input.span();
        let marker = self.static_string(span, text.input.to_static().unwrap());
        quote_spanned! {
            span =>
            dioxus_core::view::StaticTextBuilder::<#marker>(::core::marker::PhantomData)
        }
    }

    fn dynamic_node(&mut self, tokens: TokenStream2) -> TokenStream2 {
        self.dynamic_node_count += 1;
        // The `IntoDynNode` marker is inferred from the value's type.
        self.node_pushes.push(quote! {
            dioxus_core::view::push_dyn_node(&mut __dynamic, #tokens);
        });
        quote! { dioxus_core::view::DynamicNodeSlot }
    }

    fn dynamic_attr(&mut self, attr: &Attribute) -> TokenStream2 {
        self.track_dynamic_attr(attr);
        let attrs = attr.rendered_as_dynamic_attr();
        quote! { .attribute(dioxus_core::view::dynamic_attributes_builder(#attrs)) }
    }

    fn dynamic_builder_attr(&mut self, attr: &Attribute, method: Ident) -> TokenStream2 {
        self.track_dynamic_attr(attr);
        let attr_value = &attr.value;
        let method = if attr.name.is_likely_event() {
            event_handler_method(&method, attr_value)
        } else {
            method
        };
        let value = quote! { #attr_value };
        quote! { .#method(#value) }
    }

    fn track_dynamic_attr(&mut self, attr: &Attribute) {
        self.dynamic_attr_count += 1;
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
                    quote! { dioxus_core::internal::HotReloadLiteralMeta::Fmted(#fmted) }
                }
                HotLiteral::Float(value) => {
                    quote! { dioxus_core::internal::HotReloadLiteralMeta::Float(#value as _) }
                }
                HotLiteral::Int(value) => {
                    quote! { dioxus_core::internal::HotReloadLiteralMeta::Int(#value as _) }
                }
                HotLiteral::Bool(value) => {
                    quote! { dioxus_core::internal::HotReloadLiteralMeta::Bool(#value) }
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
                self.dynamic_text_tokens.push(quote! { #segment });
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

    fn element_tag(&mut self, element: &Element) -> TokenStream2 {
        match &element.name {
            ElementName::Ident(ident) => quote_spanned! { element.name.span() => html::#ident },
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
                let marker = builder.static_string(attr.span(), value.to_static().unwrap());
                return quote! {
                    .#method(
                        dioxus_core::view::StaticAttributeValueBuilder::<#marker>(
                            ::core::marker::PhantomData
                        )
                    )
                };
            }

            return builder.dynamic_builder_attr(attr, method.clone());
        }

        let Some((name, value)) = attr.as_static_str_literal() else {
            return builder.dynamic_attr(attr);
        };

        let namespace = quote!(None::<&'static str>);
        let resolved_name = name.resolved();
        let value = value.to_static().unwrap();
        builder.static_attr(
            attr.span(),
            quote! { #resolved_name },
            quote! { #value },
            namespace,
        )
    }
}

impl TemplateBody {
    pub(crate) fn split_oversized_templates(&mut self) {
        Self::split_nodes(&mut self.roots, SiblingContext::Roots);
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

        // Save the roots; template lowering derives dynamic positions from the raw op tape.
        body.roots = nodes;

        // Finally, validate the key
        body.validate_key();

        body
    }

    fn split_nodes(nodes: &mut Vec<BodyNode>, context: SiblingContext) {
        for node in nodes.iter_mut() {
            Self::split_nested_templates(node);
        }

        if nodes.len() == 1 && Self::exceeds_hard_limits(nodes, context) {
            if matches!(context, SiblingContext::ElementChildren) {
                let node = nodes.pop().unwrap();
                *nodes = vec![BodyNode::SyntheticBoundary(Box::new(TemplateBody::new(
                    vec![node],
                )))];
            }
            return;
        }

        if nodes.len() <= 1 || !Self::exceeds_hard_limits(nodes, context) {
            return;
        }

        let original = std::mem::take(nodes);
        let chunk_len = original.len().div_ceil(2);
        *nodes = original
            .chunks(chunk_len)
            .map(|chunk| {
                let mut body = TemplateBody::new(chunk.to_vec());
                body.split_oversized_templates();
                BodyNode::SyntheticBoundary(Box::new(body))
            })
            .collect();
    }

    fn split_nested_templates(node: &mut BodyNode) {
        match node {
            BodyNode::Element(element) => {
                Self::split_nodes(&mut element.children, SiblingContext::ElementChildren);
            }
            BodyNode::Component(component) => {
                component.children.split_oversized_templates();
            }
            BodyNode::ForLoop(for_loop) => {
                for_loop.body.split_oversized_templates();
            }
            BodyNode::IfChain(if_chain) => {
                Self::split_if_chain(if_chain);
            }
            BodyNode::SyntheticBoundary(body) => {
                body.split_oversized_templates();
            }
            BodyNode::Text(_) | BodyNode::RawExpr(_) => {}
        }
    }

    fn split_if_chain(if_chain: &mut IfChain) {
        if_chain.then_branch.split_oversized_templates();
        if let Some(else_if) = &mut if_chain.else_if_branch {
            Self::split_if_chain(else_if);
        }
        if let Some(else_branch) = &mut if_chain.else_branch {
            else_branch.split_oversized_templates();
        }
    }

    fn exceeds_hard_limits(nodes: &[BodyNode], context: SiblingContext) -> bool {
        if Self::path_bits_exceed_limit(nodes) {
            return true;
        }

        let stats = sibling_storage_stats(nodes);
        stats.path_overflow
            || stats.ops > TEMPLATE_STORAGE_MAX_CAP
            || stats.strings > TEMPLATE_STORAGE_MAX_CAP
            || stats.dynamic_nodes > u16::MAX as usize
            || stats.dynamic_attributes > u16::MAX as usize
            || matches!(context, SiblingContext::Roots) && nodes.len() > u16::MAX as usize
    }

    fn path_bits_exceed_limit(nodes: &[BodyNode]) -> bool {
        Self::sibling_path_bits_exceed_limit(nodes, 0)
    }

    fn sibling_path_bits_exceed_limit(nodes: &[BodyNode], parent_bits: usize) -> bool {
        for (index, node) in nodes.iter().enumerate() {
            let node_bits = parent_bits + 1 + index;
            if Self::node_path_bits_exceed_limit(node, node_bits) {
                return true;
            }
        }
        false
    }

    fn node_path_bits_exceed_limit(node: &BodyNode, node_bits: usize) -> bool {
        if node_bits > TEMPLATE_SLOT_PATH_MAX_PATH_BITS {
            return true;
        }

        match node {
            BodyNode::Element(element) => {
                Self::sibling_path_bits_exceed_limit(&element.children, node_bits)
            }
            BodyNode::Text(_)
            | BodyNode::RawExpr(_)
            | BodyNode::Component(_)
            | BodyNode::ForLoop(_)
            | BodyNode::IfChain(_)
            | BodyNode::SyntheticBoundary(_) => false,
        }
    }

    /// Normalize the Template body for rendering. If the body is completely empty, insert a placeholder node
    pub fn normalized(&self) -> Self {
        // If the nodes are completely empty, insert a placeholder node
        // Core expects at least one node in the template to make it easier to replace
        if self.is_empty() {
            // Preserve diagnostics and the assigned hot-reload template index when inserting the
            // placeholder.
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

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;

    fn dynamic_siblings(count: usize) -> Vec<BodyNode> {
        (0..count)
            .map(|_| syn::parse2(quote! { { value } }).unwrap())
            .collect()
    }

    #[test]
    fn path_bit_split_limit_matches_slot_path_payload_capacity() {
        assert_eq!(TEMPLATE_SLOT_PATH_MAX_PATH_BITS, 127);
        assert!(!TemplateBody::path_bits_exceed_limit(&dynamic_siblings(
            TEMPLATE_SLOT_PATH_MAX_PATH_BITS
        )));
        assert!(TemplateBody::path_bits_exceed_limit(&dynamic_siblings(
            TEMPLATE_SLOT_PATH_MAX_PATH_BITS + 1
        )));
    }
}
