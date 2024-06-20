use crate::{innerlude::*, HotReloadingContext};
use dioxus_core::TemplateNode;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{quote, ToTokens};
use syn::{
    parse::{Parse, ParseStream},
    spanned::Spanned,
    token::{self, Brace},
    Expr, ExprIf, Ident, LitStr, Pat, Result, Token,
};

#[derive(PartialEq, Eq, Clone, Debug, Hash)]
pub enum BodyNode {
    /// div {}
    Element(Element),

    /// Component {}
    Component(Component),

    /// "text {formatted}"
    Text(TextNode),

    /// {expr}
    RawExpr(RawExpr),

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
                dyn_idx: DynIdx::default(),
            }));
        }

        // Raw expressions need to be wrapped in braces
        if stream.peek(token::Brace) {
            return Ok(BodyNode::RawExpr(RawExpr {
                expr: stream.parse::<Expr>()?.to_token_stream(),
                dyn_idx: DynIdx::default(),
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

// Checks if an ident looks like a component
fn ident_looks_like_component(ident: &Ident) -> bool {
    let as_string = ident.to_string();
    let first_char = as_string.chars().next().unwrap();
    // Components either start with an uppercase letter or have an underscore in them
    first_char.is_ascii_uppercase() || as_string.contains('_')
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

                TemplateNode::Element {
                    tag,
                    namespace,
                    children: intern(
                        el.children
                            .iter()
                            .map(|c| c.to_template_node::<Ctx>())
                            .collect::<Vec<_>>(),
                    ),
                    attrs: intern(
                        el.merged_attributes
                            .iter()
                            .map(|attr| attr.to_template_attribute::<Ctx>(&rust_name))
                            .collect::<Vec<_>>(),
                    ),
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

    pub fn get_dyn_idx(&self) -> usize {
        match self {
            BodyNode::Text(text) => text.dyn_idx.get(),
            BodyNode::RawExpr(exp) => exp.dyn_idx.get(),
            BodyNode::Component(comp) => comp.dyn_idx.get(),
            BodyNode::ForLoop(floop) => floop.dyn_idx.get(),
            BodyNode::IfChain(chain) => chain.dyn_idx.get(),
            _ => panic!("Cannot get dyn_idx for this node"),
        }
    }

    pub fn set_dyn_idx(&self, idx: usize) {
        match self {
            BodyNode::Text(text) => text.dyn_idx.set(idx),
            BodyNode::RawExpr(exp) => exp.dyn_idx.set(idx),
            BodyNode::Component(comp) => comp.dyn_idx.set(idx),
            BodyNode::ForLoop(floop) => floop.dyn_idx.set(idx),
            BodyNode::IfChain(chain) => chain.dyn_idx.set(idx),
            _ => panic!("Cannot set dyn_idx for this node"),
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

    pub fn el_name(&self) -> &ElementName {
        match self {
            BodyNode::Element(el) => &el.name,
            _ => panic!("Element name not available for this node"),
        }
    }
}
