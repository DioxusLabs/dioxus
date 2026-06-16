//! Lower parsed RSX bodies into the typed view builder.
//!
//! `TemplateBody` owns the parsed nodes and validates root-key rules. The actual
//! template lowering happens in `FlatTemplatePieces`, which walks the body once
//! to produce the const builder expression, dynamic value expressions, and
//! hot-reload metadata for the same slots.

use self::location::DynIdx;
use crate::flat_template::FlatTemplatePieces;
use crate::*;
use proc_macro2::{Span, TokenStream as TokenStream2};
use proc_macro2_diagnostics::SpanDiagnosticExt;
use syn::parse_quote;

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

        let template = FlatTemplatePieces::from_body(&node);
        let template_definitions = template.definitions();
        let template_expr = template.view_expr();
        let dynamic_text = template.dynamic_text_tokens().iter();

        let diagnostics = &node.diagnostics;
        let index = node.template_idx.get();
        let hot_reload_mapping = template.hot_reload_template_tokens(quote! { __template });

        tokens.append_all(quote! {
            dioxus_core::Element::Ok({
                #diagnostics

                #key_warnings

                #(#template_definitions)*

                #[cfg(debug_assertions)]
                let mut __dynamic_literal_pool = dioxus_core::internal::DynamicLiteralPool::new(
                    vec![ #( #dynamic_text.to_string() ),* ],
                );

                // The key needs to be created before the dynamic nodes as it might depend on a borrowed value which gets moved into the dynamic nodes
                let __key = #key_tokens;

                // NOTE: Allocating a temporary is important to make reads within rsx drop before the value is returned
                #[allow(clippy::let_and_return)]
                let __vnodes = {
                    use dioxus_core::view::View as _;
                    dioxus_core::view::keyed(#template_expr, __key).into_vnode()
                };

                #[cfg(debug_assertions)]
                {
                    let __template = __vnodes.template;
                    let __original_template = #hot_reload_mapping;
                    let __template_read = {
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
                    // If the template has not been hot reloaded, we always use the original template
                    // Templates nested within macros may be merged because they have the same file-line-column-index
                    // They cannot be hot reloaded, so this prevents incorrect rendering
                    let __template_read = match __template_read.as_ref().map(|__template_read| __template_read.as_ref()) {
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

impl TemplateBody {
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
