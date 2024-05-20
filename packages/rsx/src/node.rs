use self::location::CallerLocation;
use super::*;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::{
    braced,
    spanned::Spanned,
    token::{self, Brace},
    Expr, ExprIf, Ident, LitStr, Pat,
};

mod attribute;
mod block;
mod component;
mod element;
mod forloop;
mod ifchain;
mod raw_expr;
mod text_node;

pub use attribute::*;
pub use block::*;
pub use body::*;
pub use component::*;
pub use element::*;
pub use forloop::*;
pub use ifchain::*;
pub use raw_expr::*;
pub use text_node::*;

#[derive(PartialEq, Eq, Clone, Debug, Hash)]
pub enum BodyNode {
    /// div {}
    Element(Element),

    /// "text {formatted}"
    Text(TextNode),

    /// {expr}
    RawExpr(RawExpr),

    /// Component {}
    Component(Component),

    /// for item in items {}
    ForLoop(ForLoop),

    /// if cond {} else if cond {} (else {}?)
    IfChain(IfChain),
}

impl Parse for BodyNode {
    fn parse(stream: ParseStream) -> Result<Self> {
        if stream.peek(LitStr) {
            return Ok(BodyNode::Text(stream.parse()?));
        }

        // Transform for loops into into_iter calls
        if stream.peek(Token![for]) {
            return Ok(BodyNode::ForLoop(stream.parse()?));
        }

        // Transform unterminated if statements into terminated optional if statements
        if stream.peek(Token![if]) {
            return Ok(BodyNode::IfChain(stream.parse()?));
        }

        // Match statements are special but have no special arm syntax
        // we could allow arm syntax if we wanted
        //
        // ```
        // match {
        //  val => div {}
        //  other_val => div {}
        // }
        // ```
        if stream.peek(Token![match]) {
            return Ok(BodyNode::RawExpr(RawExpr {
                expr: stream.parse::<Expr>()?.to_token_stream(),
                dyn_idx: CallerLocation::default(),
            }));
        }

        // Raw expressions need to be wrapped in braces
        if stream.peek(token::Brace) {
            return Ok(BodyNode::RawExpr(RawExpr {
                expr: stream.parse::<Expr>()?.to_token_stream(),
                dyn_idx: CallerLocation::default(),
            }));
        }

        // If there's an ident immediately followed by a dash, it's a web component
        // Web components support no namespacing, so just parse it as an element directly
        if stream.peek(Ident) && stream.peek2(Token![-]) {
            return Ok(BodyNode::Element(stream.parse::<Element>()?));
        }

        // this is an Element if path match of:
        //
        // - one ident
        // - followed by `{`
        // - 1st char is lowercase
        // - no underscores (reserved for components)
        //
        // example:
        // div {}
        if stream.peek(Ident) && stream.peek2(Brace) {
            let ident = stream.fork().parse::<Ident>().unwrap();
            let el_name = ident.to_string();
            let first_char = el_name.chars().next().unwrap();

            if first_char.is_ascii_lowercase() && !el_name.contains('_') {
                return Ok(BodyNode::Element(stream.parse::<Element>()?));
            }
        }

        // Otherwise this should be Component, allowed syntax:
        // - syn::Path
        // - PathArguments can only apper in last segment
        // - followed by `{` or `(`, note `(` cannot be used with one ident
        //
        // example
        // Div {}
        // ::Div {}
        // crate::Div {}
        // component {} <-- already handled by elements
        // ::component {}
        // crate::component{}
        // Input::<InputProps<'_, i32> {}
        // crate::Input::<InputProps<'_, i32> {}
        if stream.fork().parse::<syn::Path>().is_ok() {
            return Ok(BodyNode::Component(stream.parse()?));
        }

        Err(syn::Error::new(
            stream.span(),
            "Expected a valid body node.\nExpressions must be wrapped in curly braces.",
        ))
    }
}

impl ToTokens for BodyNode {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        match self {
            BodyNode::Element(ela) => ela.to_tokens(tokens),
            BodyNode::RawExpr(exp) => exp.to_tokens(tokens),
            BodyNode::Text(txt) => txt.to_tokens(tokens),
            BodyNode::ForLoop(floop) => floop.to_tokens(tokens),
            BodyNode::Component(comp) => comp.to_tokens(tokens),
            BodyNode::IfChain(ifchain) => ifchain.to_tokens(tokens),
        }
    }
}

impl BodyNode {
    /// Convert this BodyNode into a TemplateNode.
    ///
    /// dioxus-core uses this to understand templates at compiletime
    pub fn to_template_node<Ctx: HotReloadingContext>(&self) -> TemplateNode {
        match self {
            BodyNode::Element(el) => {
                let rust_name = el.name.to_string();

                let (tag, namespace) =
                    Ctx::map_element(&rust_name).unwrap_or((intern(rust_name.as_str()), None));

                let mut static_attr_array = Vec::new();

                for attr in &el.merged_attributes {
                    let template_attr = match attr.as_static_str_literal() {
                        // For static attributes, we don't need to pull in any mapping or anything
                        // We can just build them directly
                        Some((name, value)) => {
                            let value = value.source.as_ref().unwrap();
                            let attribute_name_rust = name.to_string();

                            let (name, namespace) =
                                Ctx::map_attribute(&rust_name, &attribute_name_rust)
                                    .unwrap_or((intern(attribute_name_rust.as_str()), None));

                            let static_attr = TemplateAttribute::Static {
                                name,
                                namespace,
                                value: intern(value.value().as_str()),
                            };

                            static_attr
                        }

                        // For dynamic attributes, we need to check the mapping to see if that mapping exists
                        // todo: one day we could generate new dynamic attributes on the fly if they're a literal,
                        // or something sufficiently serializable
                        //  (ie `checked`` being a bool and bools being interpretable)
                        //
                        // For now, just give up if that attribute doesn't exist in the mapping
                        None => {
                            let id = usize::MAX;
                            // let id = attr.dyn_idx.get();
                            TemplateAttribute::Dynamic { id }
                        }
                    };

                    static_attr_array.push(template_attr);
                }

                TemplateNode::Element {
                    children: el
                        .children
                        .iter()
                        .map(|c| c.to_template_node::<Ctx>())
                        .collect::<Vec<_>>()
                        .leak(),
                    tag,
                    namespace,
                    attrs: &[],
                }
            }
            BodyNode::Text(text) if text.is_static() => {
                let text = text.input.source.as_ref().unwrap();
                let text = intern(text.value().as_str());
                TemplateNode::Text { text }
            }
            BodyNode::Text(text) => TemplateNode::DynamicText {
                id: text.dyn_idx.get(),
            },
            BodyNode::RawExpr(exp) => TemplateNode::Dynamic {
                id: exp.dyn_idx.get(),
            },
            BodyNode::Component(comp) => TemplateNode::Dynamic {
                id: comp.dyn_idx.get(),
            },
            BodyNode::ForLoop(floop) => TemplateNode::Dynamic {
                id: floop.dyn_idx.get(),
            },
            BodyNode::IfChain(chain) => TemplateNode::Dynamic {
                id: chain.dyn_idx.get(),
            },
        }
    }

    pub(crate) fn set_location_idx(&self, idx: usize) {
        match self {
            BodyNode::IfChain(chain) => chain.dyn_idx.set(idx),
            BodyNode::ForLoop(floop) => floop.dyn_idx.set(idx),
            BodyNode::Component(comp) => comp.dyn_idx.set(idx),
            BodyNode::Text(text) => text.dyn_idx.set(idx),
            BodyNode::RawExpr(expr) => expr.dyn_idx.set(idx),
            BodyNode::Element(_) => todo!(),
        }
    }

    pub fn is_litstr(&self) -> bool {
        matches!(self, BodyNode::Text { .. })
    }

    pub fn span(&self) -> Span {
        match self {
            BodyNode::Element(el) => el.name.span(),
            BodyNode::Component(component) => component.name.span(),
            BodyNode::Text(text) => text.input.source.span(),
            BodyNode::RawExpr(exp) => exp.expr.span(),
            BodyNode::ForLoop(fl) => fl.for_token.span(),
            BodyNode::IfChain(f) => f.if_token.span(),
        }
    }

    pub fn children(&self) -> &[BodyNode] {
        match self {
            BodyNode::Element(el) => &el.children,
            BodyNode::Component(comp) => &comp.children.roots,
            _ => panic!("Children not available for this node"),
        }
    }
}
