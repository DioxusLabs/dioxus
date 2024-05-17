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
mod component;
mod element;
mod forloop;
mod ifchain;
mod text_node;

pub use attribute::*;
pub use body::*;
pub use component::*;
pub use element::*;
pub use forloop::*;
pub use ifchain::*;
pub use text_node::*;

#[derive(PartialEq, Eq, Clone, Debug, Hash)]
pub enum BodyNode {
    Element(Element),
    Text(TextNode),
    RawExpr(Expr),
    Component(Component),
    ForLoop(ForLoop),
    IfChain(IfChain),
}

impl BodyNode {
    pub fn is_litstr(&self) -> bool {
        matches!(self, BodyNode::Text { .. })
    }

    pub fn children(&self) -> &[BodyNode] {
        match self {
            BodyNode::Element(el) => &el.children,
            BodyNode::Component(comp) => &comp.children,
            _ => panic!("Children not available for this node"),
        }
    }

    pub fn span(&self) -> Span {
        match self {
            BodyNode::Element(el) => el.name.span(),
            BodyNode::Component(component) => component.name.span(),
            BodyNode::Text(text) => text.input.source.span(),
            BodyNode::RawExpr(exp) => exp.span(),
            BodyNode::ForLoop(fl) => fl.for_token.span(),
            BodyNode::IfChain(f) => f.if_token.span(),
        }
    }

    pub(crate) fn set_location_idx(&self, idx: usize) {
        todo!()
        // match self {
        //     BodyNode::IfChain(chain) => {
        //         chain.location.idx.set(idx);
        //     }
        //     BodyNode::ForLoop(floop) => {
        //         floop.location.idx.set(idx);
        //     }
        //     BodyNode::Component(comp) => {
        //         comp.location.idx.set(idx);
        //     }
        //     BodyNode::Text(text) => {
        //         text.location.idx.set(idx);
        //     }
        //     BodyNode::Element(_) => {}
        //     BodyNode::RawExpr(_) => {}
        // }
    }
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
            return Ok(BodyNode::RawExpr(stream.parse::<Expr>()?));
        }

        // Raw expressions need to be wrapped in braces
        if stream.peek(token::Brace) {
            return Ok(BodyNode::RawExpr(stream.parse::<Expr>()?));
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

pub(crate) fn parse_buffer_as_braced_children(
    input: &syn::parse::ParseBuffer<'_>,
) -> Result<(Brace, Vec<BodyNode>)> {
    let content;
    let brace_token = braced!(content in input);
    let mut then_branch = vec![];
    while !content.is_empty() {
        then_branch.push(content.parse()?);
    }
    Ok((brace_token, then_branch))
}
