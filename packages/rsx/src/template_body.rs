//! I'm so sorry this is so complicated. Here's my best to simplify and explain it:
//!
//! The `Callbody` is the contents of the rsx! macro - this contains all the information about every
//! node that rsx! directly knows about. For loops, if statements, etc.
//!
//! However, there are multiple *templates* inside a callbody - due to how core clones templates and
//! just generally rationalize the concept of a template, nested bodies like for loops and if statements
//! and component children are all templates, contained within the same Callbody.
//!
//! This gets confusing fast since there's lots of IDs bouncing around.
//!
//! The IDs at play:
//! - The id of the template itself so we can find it and apply it to the dom.
//!   This is challenging since all calls to file/line/col/id are relative to the macro invocation,
//!   so they will have to share the same base ID and we need to give each template a new ID.
//!   The id of the template will be something like file!():line!():col!():ID where ID increases for
//!   each nested template.
//!
//! - The IDs of dynamic nodes relative to the template they live in. This is somewhat easy to track
//!   but needs to happen on a per-template basis.
//!
//! - The IDs of formatted strings in debug mode only. Any formatted segments like "{x:?}" get pulled out
//!   into a pool so we can move them around during hot reloading on a per-template basis.
//!
//! - The IDs of component property literals in debug mode only. Any component property literals like
//!   1234 get pulled into the pool so we can hot reload them with the context of the literal pool.
//!
//! We solve this by parsing the structure completely and then doing a second pass that fills in IDs
//! by walking the structure.
//!
//! This means you can't query the ID of any node "in a vacuum" - these are assigned once - but at
//! least they're stable enough for the purposes of hotreloading
//!
//! ```rust, ignore
//! rsx! {
//!     div {
//!         class: "hello",
//!         id: "node-{node_id}",    <--- {node_id} has the formatted segment id 0 in the literal pool
//!         ..props,                 <--- spreads are not reloadable
//!
//!         "Hello, world!           <--- not tracked but reloadable in the template since it's just a string
//!
//!         for item in 0..10 {      <--- both 0 and 10 are technically reloadable, but we don't hot reload them today...
//!             div { "cool-{item}" }     <--- {item} has the formatted segment id 1 in the literal pool
//!         }
//!
//!         Link {
//!             to: "/home", <-- hotreloadable since its a component prop literal (with component literal id 0)
//!             class: "link {is_ready}", <-- {is_ready} has the formatted segment id 2 in the literal pool and the property has the component literal id 1
//!             "Home" <-- hotreloadable since its a component child (via template)
//!         }
//!     }
//! }
//! ```

use self::location::DynIdx;
use crate::innerlude::Attribute;
use crate::*;
use proc_macro2::TokenStream as TokenStream2;
use proc_macro2_diagnostics::SpanDiagnosticExt;
use syn::parse_quote;

type NodePath = Vec<u8>;
type AttributePath = Vec<u8>;

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
    pub node_paths: Vec<NodePath>,
    pub attr_paths: Vec<(AttributePath, usize)>,
    pub dynamic_text_segments: Vec<FormattedSegment>,
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

        let roots = node.quote_roots();

        // Print paths is easy - just print the paths
        let node_paths = node.node_paths.iter().map(|it| quote!(&[#(#it),*]));
        let attr_paths = node.attr_paths.iter().map(|(it, _)| quote!(&[#(#it),*]));

        // For printing dynamic nodes, we rely on the ToTokens impl
        // Elements have a weird ToTokens - they actually are the entrypoint for Template creation
        let dynamic_nodes: Vec<_> = node.dynamic_nodes().collect();
        let dynamic_nodes_len = dynamic_nodes.len();

        // We could add a ToTokens for Attribute but since we use that for both components and elements
        // They actually need to be different, so we just localize that here
        let dyn_attr_printer: Vec<_> = node
            .dynamic_attributes()
            .map(|attr| attr.rendered_as_dynamic_attr())
            .collect();
        let dynamic_attr_len = dyn_attr_printer.len();

        let dynamic_text = node.dynamic_text_segments.iter();

        let diagnostics = &node.diagnostics;
        let index = node.template_idx.get();
        let hot_reload_mapping = node.hot_reload_mapping();

        tokens.append_all(quote! {
            dioxus_core::Element::Ok({
                #diagnostics

                // Components pull in the dynamic literal pool and template in debug mode, so they need to be defined before dynamic nodes
                #[cfg(debug_assertions)]
                fn __original_template() -> &'static dioxus_core::internal::HotReloadedTemplate {
                    static __ORIGINAL_TEMPLATE: ::std::sync::OnceLock<dioxus_core::internal::HotReloadedTemplate> = ::std::sync::OnceLock::new();
                    if __ORIGINAL_TEMPLATE.get().is_none() {
                        _ = __ORIGINAL_TEMPLATE.set(#hot_reload_mapping);
                    }
                    __ORIGINAL_TEMPLATE.get().unwrap()
                }
                #[cfg(debug_assertions)]
                let __template_read = {
                    static __NORMALIZED_FILE: &'static str = {
                        const PATH: &str = dioxus_core::const_format::str_replace!(file!(), "\\\\", "/");
                        dioxus_core::const_format::str_replace!(PATH, '\\', "/")
                    };

                    // The key is important here - we're creating a new GlobalSignal each call to this
                    // But the key is what's keeping it stable
                    static __TEMPLATE: GlobalSignal<Option<dioxus_core::internal::HotReloadedTemplate>> = GlobalSignal::with_location(
                        || None::<dioxus_core::internal::HotReloadedTemplate>,
                        __NORMALIZED_FILE,
                        line!(),
                        column!(),
                        #index
                    );

                    dioxus_core::Runtime::current().ok().map(|_| __TEMPLATE.read())
                };
                // If the template has not been hot reloaded, we always use the original template
                // Templates nested within macros may be merged because they have the same file-line-column-index
                // They cannot be hot reloaded, so this prevents incorrect rendering
                #[cfg(debug_assertions)]
                let __template_read = match __template_read.as_ref().map(|__template_read| __template_read.as_ref()) {
                    Some(Some(__template_read)) => &__template_read,
                    _ => __original_template(),
                };
                #[cfg(debug_assertions)]
                let mut __dynamic_literal_pool = dioxus_core::internal::DynamicLiteralPool::new(
                    vec![ #( #dynamic_text.to_string() ),* ],
                );

                // These items are used in both the debug and release expansions of rsx. Pulling them out makes the expansion
                // slightly smaller and easier to understand. Rust analyzer also doesn't autocomplete well when it sees an ident show up twice in the expansion
                let __dynamic_nodes: [dioxus_core::DynamicNode; #dynamic_nodes_len] = [ #( #dynamic_nodes ),* ];
                let __dynamic_attributes: [Box<[dioxus_core::Attribute]>; #dynamic_attr_len] = [ #( #dyn_attr_printer ),* ];
                #[doc(hidden)]
                static __TEMPLATE_ROOTS: &[dioxus_core::TemplateNode] = &[ #( #roots ),* ];

                #[cfg(debug_assertions)]
                {
                    let mut __dynamic_value_pool = dioxus_core::internal::DynamicValuePool::new(
                        Vec::from(__dynamic_nodes),
                        Vec::from(__dynamic_attributes),
                        __dynamic_literal_pool
                    );
                    __dynamic_value_pool.render_with(__template_read)
                }
                #[cfg(not(debug_assertions))]
                {
                    #[doc(hidden)] // vscode please stop showing these in symbol search
                    static ___TEMPLATE: dioxus_core::Template = dioxus_core::Template {
                        roots: __TEMPLATE_ROOTS,
                        node_paths: &[ #( #node_paths ),* ],
                        attr_paths: &[ #( #attr_paths ),* ],
                    };

                    // NOTE: Allocating a temporary is important to make reads within rsx drop before the value is returned
                    #[allow(clippy::let_and_return)]
                    let __vnodes = dioxus_core::VNode::new(
                        #key_tokens,
                        ___TEMPLATE,
                        Box::new(__dynamic_nodes),
                        Box::new(__dynamic_attributes),
                    );
                    __vnodes
                }
            })
        });
    }
}

impl TemplateBody {
    /// Create a new TemplateBody from a set of nodes
    ///
    /// This will fill in all the necessary path information for the nodes in the template and will
    /// overwrite data like dynamic indexes.
    pub fn new(nodes: Vec<BodyNode>) -> Self {
        let mut body = Self {
            roots: vec![],
            template_idx: DynIdx::default(),
            node_paths: Vec::new(),
            attr_paths: Vec::new(),
            dynamic_text_segments: Vec::new(),
            diagnostics: Diagnostics::new(),
        };

        // Assign paths to all nodes in the template
        body.assign_paths_inner(&nodes);
        body.validate_key();

        // And then save the roots
        body.roots = nodes;

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
        match self.roots.first() {
            Some(BodyNode::Element(el)) => el.key(),
            Some(BodyNode::Component(comp)) => comp.get_key(),
            _ => None,
        }
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

    pub fn get_dyn_node(&self, path: &[u8]) -> &BodyNode {
        let mut node = self.roots.get(path[0] as usize).unwrap();
        for idx in path.iter().skip(1) {
            node = node.element_children().get(*idx as usize).unwrap();
        }
        node
    }

    pub fn get_dyn_attr(&self, path: &AttributePath, idx: usize) -> &Attribute {
        match self.get_dyn_node(path) {
            BodyNode::Element(el) => &el.merged_attributes[idx],
            _ => unreachable!(),
        }
    }

    pub fn dynamic_attributes(&self) -> impl DoubleEndedIterator<Item = &Attribute> {
        self.attr_paths
            .iter()
            .map(|(path, idx)| self.get_dyn_attr(path, *idx))
    }

    pub fn dynamic_nodes(&self) -> impl DoubleEndedIterator<Item = &BodyNode> {
        self.node_paths.iter().map(|path| self.get_dyn_node(path))
    }

    fn quote_roots(&self) -> impl Iterator<Item = TokenStream2> + '_ {
        self.roots.iter().map(|node| match node {
            BodyNode::Element(el) => quote! { #el },
            BodyNode::Text(text) if text.is_static() => {
                let text = text.input.to_static().unwrap();
                quote! { dioxus_core::TemplateNode::Text { text: #text } }
            }
            _ => {
                let id = node.get_dyn_idx();
                quote! { dioxus_core::TemplateNode::Dynamic { id: #id } }
            }
        })
    }

    /// Iterate through the literal component properties of this rsx call in depth-first order
    pub fn literal_component_properties(&self) -> impl Iterator<Item = &HotLiteral> + '_ {
        self.dynamic_nodes()
            .filter_map(|node| {
                if let BodyNode::Component(component) = node {
                    Some(component)
                } else {
                    None
                }
            })
            .flat_map(|component| {
                component.component_props().filter_map(|field| {
                    if let AttributeValue::AttrLiteral(literal) = &field.value {
                        Some(literal)
                    } else {
                        None
                    }
                })
            })
    }

    fn hot_reload_mapping(&self) -> TokenStream2 {
        let key = if let Some(AttributeValue::AttrLiteral(HotLiteral::Fmted(key))) =
            self.implicit_key()
        {
            quote! { Some(#key) }
        } else {
            quote! { None }
        };
        let dynamic_nodes = self.dynamic_nodes().map(|node| {
            let id = node.get_dyn_idx();
            quote! { dioxus_core::internal::HotReloadDynamicNode::Dynamic(#id) }
        });
        let dyn_attr_printer = self.dynamic_attributes().map(|attr| {
            let id = attr.get_dyn_idx();
            quote! { dioxus_core::internal::HotReloadDynamicAttribute::Dynamic(#id) }
        });
        let component_values = self
            .literal_component_properties()
            .map(|literal| literal.quote_as_hot_reload_literal());
        quote! {
            dioxus_core::internal::HotReloadedTemplate::new(
                #key,
                vec![ #( #dynamic_nodes ),* ],
                vec![ #( #dyn_attr_printer ),* ],
                vec![ #( #component_values ),* ],
                __TEMPLATE_ROOTS,
            )
        }
    }
}
