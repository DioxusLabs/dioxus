//!
//! TODO:
//! - [ ] Support for VComponents
//! - [ ] Support for inline format in text
//! - [ ] Support for expressions in attribute positions
//! - [ ] Support for iterators
//! - [ ] support for inline html!
//!
//!
//!
//!
//!
//!
//!

use {
    proc_macro::TokenStream,
    proc_macro2::{Span, TokenStream as TokenStream2},
    quote::{quote, ToTokens, TokenStreamExt},
    syn::{
        ext::IdentExt,
        parse::{Parse, ParseStream},
        token, Error, Expr, ExprClosure, Ident, LitBool, LitStr, Path, Result, Token,
    },
};

// ==============================================
// Parse any stream coming from the html! macro
// ==============================================
pub struct HtmlRender {
    kind: NodeOrList,
}

impl Parse for HtmlRender {
    fn parse(s: ParseStream) -> Result<Self> {
        // let ctx: Ident = s.parse()?;
        // s.parse::<Token![,]>()?;
        // if elements are in an array, return a bumpalo::collections::Vec rather than a Node.
        let kind = if s.peek(token::Bracket) {
            let nodes_toks;
            syn::bracketed!(nodes_toks in s);
            let mut nodes: Vec<MaybeExpr<Node>> = vec![nodes_toks.parse()?];
            while nodes_toks.peek(Token![,]) {
                nodes_toks.parse::<Token![,]>()?;
                nodes.push(nodes_toks.parse()?);
            }
            NodeOrList::List(NodeList(nodes))
        } else {
            NodeOrList::Node(s.parse()?)
        };
        Ok(HtmlRender { kind })
    }
}

impl ToTokens for HtmlRender {
    fn to_tokens(&self, out_tokens: &mut TokenStream2) {
        let new_toks = ToToksCtx::new(&self.kind).to_token_stream();

        // create a lazy tree that accepts a bump allocator
        let final_tokens = quote! {
            dioxus::prelude::LazyNodes::new(move |ctx| {
                let bump = &ctx.bump();

                #new_toks
            })
        };

        final_tokens.to_tokens(out_tokens);
    }
}

/// =============================================
/// Parse any child as a node or list of nodes
/// =============================================
/// - [ ] Allow iterators
///
///
enum NodeOrList {
    Node(Node),
    List(NodeList),
}

impl ToTokens for ToToksCtx<&NodeOrList> {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        match self.inner {
            NodeOrList::Node(node) => self.recurse(node).to_tokens(tokens),
            NodeOrList::List(list) => self.recurse(list).to_tokens(tokens),
        }
    }
}

struct NodeList(Vec<MaybeExpr<Node>>);

impl ToTokens for ToToksCtx<&NodeList> {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let nodes = self.inner.0.iter().map(|node| self.recurse(node));
        tokens.append_all(quote! {
            dioxus::bumpalo::vec![in bump;
                #(#nodes),*
            ]
        });
    }
}

enum Node {
    Element(Element),
    Text(TextNode),
}

impl ToTokens for ToToksCtx<&Node> {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        match &self.inner {
            Node::Element(el) => self.recurse(el).to_tokens(tokens),
            Node::Text(txt) => self.recurse(txt).to_tokens(tokens),
        }
    }
}

impl Node {
    fn peek(s: ParseStream) -> bool {
        (s.peek(Token![<]) && !s.peek2(Token![/])) || s.peek(token::Brace) || s.peek(LitStr)
    }
}

impl Parse for Node {
    fn parse(s: ParseStream) -> Result<Self> {
        Ok(if s.peek(Token![<]) {
            Node::Element(s.parse()?)
        } else {
            Node::Text(s.parse()?)
        })
    }
}

/// =======================================
/// Parse the VNode::Element type
/// =======================================
/// - [ ] Allow VComponent
///
///
struct Element {
    name: Ident,
    attrs: Vec<Attr>,
    children: MaybeExpr<Vec<Node>>,
}

impl ToTokens for ToToksCtx<&Element> {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        // let ctx = self.ctx;
        let _name = &self.inner.name;
        tokens.append_all(quote! {
            dioxus::builder::ElementBuilder::new(ctx, "#name")
        });
        for attr in self.inner.attrs.iter() {
            self.recurse(attr).to_tokens(tokens);
        }
        match &self.inner.children {
            MaybeExpr::Expr(expr) => tokens.append_all(quote! {
                .children(#expr)
            }),
            MaybeExpr::Literal(nodes) => {
                let mut children = nodes.iter();
                if let Some(child) = children.next() {
                    let mut inner_toks = TokenStream2::new();
                    self.recurse(child).to_tokens(&mut inner_toks);
                    while let Some(child) = children.next() {
                        quote!(,).to_tokens(&mut inner_toks);
                        self.recurse(child).to_tokens(&mut inner_toks);
                    }
                    tokens.append_all(quote! {
                        .children([#inner_toks])
                    });
                }
            }
        }
        tokens.append_all(quote! {
            .finish()
        });
    }
}

impl Parse for Element {
    fn parse(s: ParseStream) -> Result<Self> {
        s.parse::<Token![<]>()?;
        let name = Ident::parse_any(s)?;
        let mut attrs = vec![];
        let _children: Vec<Node> = vec![];

        // keep looking for attributes
        while !s.peek(Token![>]) {
            // self-closing
            if s.peek(Token![/]) {
                s.parse::<Token![/]>()?;
                s.parse::<Token![>]>()?;
                return Ok(Self {
                    name,
                    attrs,
                    children: MaybeExpr::Literal(vec![]),
                });
            }
            attrs.push(s.parse()?);
        }
        s.parse::<Token![>]>()?;

        // Contents of an element can either be a brace (in which case we just copy verbatim), or a
        // sequence of nodes.
        let children = if s.peek(token::Brace) {
            // expr
            let content;
            syn::braced!(content in s);
            MaybeExpr::Expr(content.parse()?)
        } else {
            // nodes
            let mut children = vec![];
            while !(s.peek(Token![<]) && s.peek2(Token![/])) {
                children.push(s.parse()?);
            }
            MaybeExpr::Literal(children)
        };

        // closing element
        s.parse::<Token![<]>()?;
        s.parse::<Token![/]>()?;
        let close = Ident::parse_any(s)?;
        if close.to_string() != name.to_string() {
            return Err(Error::new_spanned(
                close,
                "closing element does not match opening",
            ));
        }
        s.parse::<Token![>]>()?;
        Ok(Self {
            name,
            attrs,
            children,
        })
    }
}

/// =======================================
/// Parse a VElement's Attributes
/// =======================================
/// - [ ] Allow expressions as attribute
///
///
struct Attr {
    name: Ident,
    ty: AttrType,
}

impl Parse for Attr {
    fn parse(s: ParseStream) -> Result<Self> {
        let mut name = Ident::parse_any(s)?;
        let name_str = name.to_string();
        s.parse::<Token![=]>()?;

        // Check if this is an event handler
        // If so, parse into literal tokens
        let ty = if name_str.starts_with("on") {
            // remove the "on" bit
            name = Ident::new(&name_str.trim_start_matches("on"), name.span());
            let content;
            syn::braced!(content in s);
            // AttrType::Value(content.parse()?)
            AttrType::Event(content.parse()?)
        // AttrType::Event(content.parse()?)
        } else {
            let lit_str = if name_str == "style" && s.peek(token::Brace) {
                // special-case to deal with literal styles.
                let outer;
                syn::braced!(outer in s);
                // double brace for inline style.
                // todo!("Style support not ready yet");

                // if outer.peek(token::Brace) {
                //     let inner;
                //     syn::braced!(inner in outer);
                //     let styles: Styles = inner.parse()?;
                //     MaybeExpr::Literal(LitStr::new(&styles.to_string(), Span::call_site()))
                // } else {
                // just parse as an expression
                MaybeExpr::Expr(outer.parse()?)
            // }
            } else {
                s.parse()?
            };
            AttrType::Value(lit_str)
        };
        Ok(Attr { name, ty })
    }
}

impl ToTokens for ToToksCtx<&Attr> {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let name = self.inner.name.to_string();
        let _attr_stream = TokenStream2::new();
        match &self.inner.ty {
            AttrType::Value(value) => {
                let value = self.recurse(value);
                tokens.append_all(quote! {
                    .attr(#name, #value)
                });
            }
            AttrType::Event(event) => {
                tokens.append_all(quote! {
                    .on(#name, #event)
                });
            }
        }
    }
}

enum AttrType {
    Value(MaybeExpr<LitStr>),
    Event(ExprClosure),
    // todo Bool(MaybeExpr<LitBool>)
}

/// =======================================
/// Parse just plain text
/// =======================================
/// - [ ] Perform formatting automatically
///
///
struct TextNode(MaybeExpr<LitStr>);

impl Parse for TextNode {
    fn parse(s: ParseStream) -> Result<Self> {
        Ok(Self(s.parse()?))
    }
}

impl ToTokens for ToToksCtx<&TextNode> {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let mut token_stream = TokenStream2::new();
        self.recurse(&self.inner.0).to_tokens(&mut token_stream);
        tokens.append_all(quote! {
            {
                use bumpalo::core_alloc::fmt::Write;
                let mut s = bumpalo::collections::String::new_in(bump);
                s.write_fmt(format_args_f!(#token_stream)).unwrap();
                dioxus::builder::text2(s)
            }
        });
    }
}

enum MaybeExpr<T> {
    Literal(T),
    Expr(Expr),
}

impl<T: Parse> Parse for MaybeExpr<T> {
    fn parse(s: ParseStream) -> Result<Self> {
        if s.peek(token::Brace) {
            let content;
            syn::braced!(content in s);
            Ok(MaybeExpr::Expr(content.parse()?))
        } else {
            Ok(MaybeExpr::Literal(s.parse()?))
        }
    }
}

impl<'a, T> ToTokens for ToToksCtx<&'a MaybeExpr<T>>
where
    T: 'a,
    ToToksCtx<&'a T>: ToTokens,
{
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        match &self.inner {
            MaybeExpr::Literal(v) => self.recurse(v).to_tokens(tokens),
            MaybeExpr::Expr(expr) => expr.to_tokens(tokens),
        }
    }
}

/// ToTokens context
struct ToToksCtx<T> {
    inner: T,
}

impl<'a, T> ToToksCtx<T> {
    fn new(inner: T) -> Self {
        ToToksCtx { inner }
    }

    fn recurse<U>(&self, inner: U) -> ToToksCtx<U> {
        ToToksCtx { inner }
    }
}

impl ToTokens for ToToksCtx<&LitStr> {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        self.inner.to_tokens(tokens)
    }
}

#[cfg(test)]
mod test {
    fn parse(input: &str) -> super::Result<super::HtmlRender> {
        syn::parse_str(input)
    }

    #[test]
    fn div() {
        parse("bump, <div class=\"test\"/>").unwrap();
    }

    #[test]
    fn nested() {
        parse("bump, <div class=\"test\"><div />\"text\"</div>").unwrap();
    }

    #[test]
    fn complex() {
        parse(
            "bump,
            <section style={{
                display: flex;
                flex-direction: column;
                max-width: 95%;
            }} class=\"map-panel\">{contact_details}</section>
        ",
        )
        .unwrap();
    }
}
