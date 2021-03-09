/*


https://github.com/chinedufn/percy/issues/37

An example usage of rsx! would look like this:
```ignore
ctx.render(rsx!{
    div {
        h1 { "Example" },
        p {
            tag: "type",
            abc: 123,
            enabled: true,
            class: "big small wide short",

            a { "abcder" },
            h2 { "whatsup", class: "abc-123" },
            CustomComponent { a: 123, b: 456, key: "1" },
            { 0..3.map(|i| rsx!{ h1 {"{:i}"} }) },
            {expr}

            // expr can take:
            // - iterator
            // - |bump| { }
            // - value (gets formatted as &str)
            // - ... more as we upgrade it
        }
    }
})
```

each element is given by tag { [Attr] }

*/
use syn::parse::{discouraged::Speculative, ParseBuffer};

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
// Parse any stream coming from the rsx! macro
// ==============================================
pub struct RsxRender {
    el: Element,
}

impl Parse for RsxRender {
    fn parse(input: ParseStream) -> Result<Self> {
        // cannot accept multiple elements
        // can only accept one root per component
        // fragments can be used as
        // todo
        // enable fragements by autocoerrcing into list
        let el: Element = input.parse()?;
        Ok(Self { el })
    }
}

impl ToTokens for RsxRender {
    fn to_tokens(&self, out_tokens: &mut TokenStream2) {
        let new_toks = ToToksCtx::new(&self.el).to_token_stream();
        // let new_toks = ToToksCtx::new(&self.kind).to_token_stream();

        // create a lazy tree that accepts a bump allocator
        let final_tokens = quote! {
            move |ctx| {
                let bump = ctx.bump;
                #new_toks
            }
        };

        final_tokens.to_tokens(out_tokens);
    }
}

// ==============================================
// Parse any div {} as a VElement
// ==============================================
enum Node {
    Element(Element),
    Text(TextNode),
    Component(Component),
}

impl ToTokens for ToToksCtx<&Node> {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        match &self.inner {
            Node::Element(el) => self.recurse(el).to_tokens(tokens),
            Node::Text(txt) => self.recurse(txt).to_tokens(tokens),
            Node::Component(c) => self.recurse(c).to_tokens(tokens),
        }
    }
}

impl Parse for Node {
    fn parse(s: ParseStream) -> Result<Self> {
        let fork1 = s.fork();
        let fork2 = s.fork();

        // todo: map issues onto the second fork if any arise
        // it's unlikely that textnodes or components would fail?

        let ret = if let Ok(text) = fork1.parse::<TextNode>() {
            s.advance_to(&fork1);
            Self::Text(text)
        } else if let Ok(element) = fork2.parse::<Element>() {
            s.advance_to(&fork2);
            Self::Element(element)
        } else if let Ok(comp) = s.parse::<Component>() {
            Self::Component(comp)
        } else {
            return Err(Error::new(s.span(), "Failed to parse as a valid child"));
        };

        // consume comma if it exists
        // we don't actually care if there *are* commas after elements/text
        if s.peek(Token![,]) {
            let _ = s.parse::<Token![,]>();
        }
        Ok(ret)
    }
}

struct Component {
    name: Ident,
    body: Vec<ComponentField>,
    // attrs: Vec<Attr>,
    children: MaybeExpr<Vec<Node>>,
}

impl ToTokens for ToToksCtx<&Component> {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let name = &self.inner.name;

        // let mut toks = quote! {};

        // for attr in self.inner.attrs.iter() {
        //     self.recurse(attr).to_tokens(&mut toks);
        // }

        let mut builder = quote! {
            fc_to_builder(#name)
        };

        for field in &self.inner.body {
            builder.append_all(quote! {#field});
        }

        builder.append_all(quote! {
            .build()
        });

        // panic!("tokens are {:#?}", toks);

        // no children right now

        // match &self.inner.children {
        //     MaybeExpr::Expr(expr) => tokens.append_all(quote! {
        //         .children(#expr)
        //     }),
        //     MaybeExpr::Literal(nodes) => {
        //         let mut children = nodes.iter();
        //         if let Some(child) = children.next() {
        //             let mut inner_toks = TokenStream2::new();
        //             self.recurse(child).to_tokens(&mut inner_toks);
        //             while let Some(child) = children.next() {
        //                 quote!(,).to_tokens(&mut inner_toks);
        //                 self.recurse(child).to_tokens(&mut inner_toks);
        //             }
        //             tokens.append_all(quote! {
        //                 .children([#inner_toks])
        //             });
        //         }
        //     }
        // }
        // tokens.append_all(quote! {
        //     .finish()
        // });
        let toks = tokens.append_all(quote! {
            dioxus::builder::virtual_child(ctx, #name, #builder)
        });
    }
}

impl Parse for Component {
    fn parse(s: ParseStream) -> Result<Self> {
        // TODO: reject anything weird/nonstandard
        // we want names *only*
        let name = Ident::parse_any(s)?;

        if crate::util::is_valid_tag(&name.to_string()) {
            return Err(Error::new(
                name.span(),
                "Components cannot share names with valid HTML tags",
            ));
        }

        // parse the guts
        let content: ParseBuffer;
        syn::braced!(content in s);

        let mut body: Vec<ComponentField> = Vec::new();
        let mut children: Vec<Node> = Vec::new();

        // parse_element_content(content, &mut attrs, &mut children);

        'parsing: loop {
            // [1] Break if empty
            if content.is_empty() {
                break 'parsing;
            }

            if let Ok(field) = content.parse::<ComponentField>() {
                body.push(field);
            }
        }

        // let body = content.parse()?;

        // eventually we'll parse the attrs
        // let mut attrs: Vec<Attr> = vec![];
        let mut children: Vec<Node> = vec![];
        // parse_element_content(content, &mut attrs, &mut children);

        let children = MaybeExpr::Literal(children);

        Ok(Self {
            name,
            body,
            // attrs,
            children,
        })
    }
}

// the struct's fields info
pub struct ComponentField {
    name: Ident,
    content: Expr,
}

impl Parse for ComponentField {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut name = Ident::parse_any(input)?;
        let name_str = name.to_string();
        input.parse::<Token![:]>()?;
        let content = input.parse()?;

        // consume comma if it exists
        // we don't actually care if there *are* commas between attrs
        if input.peek(Token![,]) {
            let _ = input.parse::<Token![,]>();
        }

        Ok(Self { name, content })
    }
}

impl ToTokens for &ComponentField {
    // impl ToTokens for ToToksCtx<&ComponentField> {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let name = &self.name;
        let content = &self.content;
        tokens.append_all(quote! {
            .#name(#content)
        })
    }
}

// =======================================
// Parse the VNode::Element type
// =======================================
// - [ ] Allow VComponent
//
//
struct Element {
    name: Ident,
    attrs: Vec<Attr>,
    children: MaybeExpr<Vec<Node>>,
}

impl ToTokens for ToToksCtx<&Element> {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let name = &self.inner.name.to_string();

        tokens.append_all(quote! {
            dioxus::builder::ElementBuilder::new(ctx, #name)
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
        // TODO: reject anything weird/nonstandard
        // we want names *only*
        let name = Ident::parse_any(s)?;

        if !crate::util::is_valid_tag(&name.to_string()) {
            return Err(Error::new(name.span(), "Not a valid Html tag"));
        }

        // parse the guts
        let content: ParseBuffer;
        syn::braced!(content in s);

        let mut attrs: Vec<Attr> = vec![];
        let mut children: Vec<Node> = vec![];
        parse_element_content(content, &mut attrs, &mut children);

        let children = MaybeExpr::Literal(children);

        Ok(Self {
            name,
            attrs,
            children,
        })
    }
}

// used by both vcomponet and velement to parse the contents of the elements into attras and children
fn parse_element_content(content: ParseBuffer, attrs: &mut Vec<Attr>, children: &mut Vec<Node>) {
    'parsing: loop {
        // todo move this around into a more functional style
        // [1] Break if empty
        // [2] Try to consume an attr (with comma)
        // [3] Try to consume a child node (with comma)
        // [4] Try to consume brackets as anything thats Into<Node>
        // [last] Fail if none worked

        // [1] Break if empty
        if content.is_empty() {
            break 'parsing;
        }

        // [2] Try to consume an attr
        let fork = content.fork();
        if let Ok(attr) = fork.parse::<Attr>() {
            // make sure to advance or your computer will become a spaceheater :)
            content.advance_to(&fork);
            attrs.push(attr);
            continue 'parsing;
        }

        // [3] Try to consume a child node
        let fork = content.fork();
        if let Ok(node) = fork.parse::<Node>() {
            // make sure to advance or your computer will become a spaceheater :)
            content.advance_to(&fork);
            children.push(node);
            continue 'parsing;
        }

        // [4] TODO: Parsing brackets
        // let fork = content.fork();
        // if let Ok(el) = fork.parse() {
        //     children.push(el);
        //     continue 'parsing;
        // }

        // todo: pass a descriptive error onto the offending tokens
        panic!("Entry is not an attr or element\n {}", content)
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
        s.parse::<Token![:]>()?;

        // Check if this is an event handler
        // If so, parse into literal tokens
        let ty = if name_str.starts_with("on") {
            // remove the "on" bit
            name = Ident::new(&name_str.trim_start_matches("on"), name.span());

            if s.peek(token::Brace) {
                let content;
                syn::braced!(content in s);

                // Try to parse directly as a closure
                let fork = content.fork();
                if let Ok(event) = fork.parse::<ExprClosure>() {
                    content.advance_to(&fork);
                    AttrType::Event(event)
                } else {
                    AttrType::Tok(content.parse()?)
                }
            } else {
                AttrType::Event(s.parse()?)
            }
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

        // consume comma if it exists
        // we don't actually care if there *are* commas between attrs
        if s.peek(Token![,]) {
            let _ = s.parse::<Token![,]>();
        }

        Ok(Attr { name, ty })
    }
}

impl ToTokens for ToToksCtx<&Attr> {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let name = self.inner.name.to_string();
        let nameident = &self.inner.name;
        let mut attr_stream = TokenStream2::new();
        match &self.inner.ty {
            AttrType::Value(value) => {
                let value = self.recurse(value);
                tokens.append_all(quote! {
                    .attr(#name, #value)
                });
            }
            AttrType::Event(event) => {
                tokens.append_all(quote! {
                    .add_listener(dioxus::events::on::#nameident(ctx, #event))
                });
                // .on(#name, #event)
            }
            AttrType::Tok(exp) => {
                tokens.append_all(quote! {
                    .add_listener(dioxus::events::on::#nameident(ctx, #exp))
                });
                // .on(#name, #exp)
            }
        }
    }
}

enum AttrType {
    Value(MaybeExpr<LitStr>),
    Event(ExprClosure),
    Tok(Expr),
    // todo Bool(MaybeExpr<LitBool>)
}

// =======================================
// Parse just plain text
// =======================================
// - [ ] Perform formatting automatically
//
//
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
