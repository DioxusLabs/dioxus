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
    custom_context: Option<Ident>,
    root: RootOption,
}

enum RootOption {
    // for lists of components
    Fragment(),
    Element(Element),
    Component(Component),
}

impl ToTokens for RootOption {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        match self {
            RootOption::Fragment() => todo!(),
            RootOption::Element(el) => el.to_tokens(tokens),
            RootOption::Component(comp) => comp.to_tokens(tokens),
        }
    }
}

impl Parse for RsxRender {
    fn parse(input: ParseStream) -> Result<Self> {
        let fork = input.fork();

        let custom_context = fork
            .parse::<Ident>()
            .and_then(|ident| {
                fork.parse::<Token![,]>().map(|_| {
                    input.advance_to(&fork);
                    ident
                })
            })
            .ok();

        let forked = input.fork();
        let name = forked.parse::<Ident>()?;

        let root = match crate::util::is_valid_tag(&name.to_string()) {
            true => input.parse::<Element>().map(|el| RootOption::Element(el)),
            false => input.parse::<Component>().map(|c| RootOption::Component(c)),
        }?;

        Ok(Self {
            root,
            custom_context,
        })
    }
}

impl ToTokens for RsxRender {
    fn to_tokens(&self, out_tokens: &mut TokenStream2) {
        let new_toks = (&self.root).to_token_stream();

        // create a lazy tree that accepts a bump allocator
        let final_tokens = match &self.custom_context {
            Some(ident) => quote! {
                #ident.render(dioxus::prelude::LazyNodes::new(move |ctx|{
                    let bump = ctx.bump;
                    #new_toks
                }))
            },
            None => quote! {
                dioxus::prelude::LazyNodes::new(move |ctx|{
                    let bump = ctx.bump;
                    #new_toks
                })
            },
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
    RawExpr(Expr),
}

impl ToTokens for &Node {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        match &self {
            Node::Element(el) => el.to_tokens(tokens),
            Node::Text(txt) => txt.to_tokens(tokens),
            Node::Component(c) => c.to_tokens(tokens),
            Node::RawExpr(exp) => exp.to_tokens(tokens),
        }
    }
}

impl Parse for Node {
    fn parse(stream: ParseStream) -> Result<Self> {
        // Supposedly this approach is discouraged due to inability to return proper errors
        // TODO: Rework this to provide more informative errors

        let fork = stream.fork();
        if let Ok(text) = fork.parse::<TextNode>() {
            stream.advance_to(&fork);
            return Ok(Self::Text(text));
        }

        let fork = stream.fork();
        if let Ok(element) = fork.parse::<Element>() {
            stream.advance_to(&fork);
            return Ok(Self::Element(element));
        }

        let fork = stream.fork();
        if let Ok(comp) = fork.parse::<Component>() {
            stream.advance_to(&fork);
            return Ok(Self::Component(comp));
        }

        let fork = stream.fork();
        if let Ok(tok) = try_parse_bracketed(&fork) {
            stream.advance_to(&fork);
            return Ok(Node::RawExpr(tok));
        }

        return Err(Error::new(stream.span(), "Failed to parse as a valid node"));
    }
}

struct Component {
    name: Ident,
    body: Vec<ComponentField>,
    children: Vec<Node>,
}

impl ToTokens for &Component {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let name = &self.name;

        let mut builder = quote! {
            fc_to_builder(#name)
        };

        for field in &self.body {
            builder.append_all(quote! {#field});
        }

        builder.append_all(quote! {
            .build()
        });

        let _toks = tokens.append_all(quote! {
            dioxus::builder::virtual_child(ctx, #name, #builder)
        });
    }
}

impl Parse for Component {
    fn parse(s: ParseStream) -> Result<Self> {
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
        let _children: Vec<Node> = Vec::new();

        'parsing: loop {
            // [1] Break if empty
            if content.is_empty() {
                break 'parsing;
            }

            if let Ok(field) = content.parse::<ComponentField>() {
                body.push(field);
            }

            // consume comma if it exists
            // we don't actually care if there *are* commas between attrs
            if content.peek(Token![,]) {
                let _ = content.parse::<Token![,]>();
            }
        }

        // todo: add support for children
        let children: Vec<Node> = vec![];

        Ok(Self {
            name,
            body,
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
        let name = Ident::parse_any(input)?;
        input.parse::<Token![:]>()?;
        let content = input.parse()?;

        Ok(Self { name, content })
    }
}

impl ToTokens for &ComponentField {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let ComponentField { name, content, .. } = self;
        tokens.append_all(quote! {
            .#name(#content)
        })
    }
}

// =======================================
// Parse the VNode::Element type
// =======================================
struct Element {
    name: Ident,
    attrs: Vec<Attr>,
    children: Vec<Node>,
}

impl ToTokens for &Element {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let name = &self.name.to_string();

        tokens.append_all(quote! {
            dioxus::builder::ElementBuilder::new(ctx, #name)
        });

        for attr in self.attrs.iter() {
            attr.to_tokens(tokens);
        }

        let mut children = self.children.iter();
        while let Some(child) = children.next() {
            let inner_toks = child.to_token_stream();
            tokens.append_all(quote! {
                .iter_child(#inner_toks)
            })
        }

        tokens.append_all(quote! {
            .finish()
        });
    }
}

impl Parse for Element {
    fn parse(s: ParseStream) -> Result<Self> {
        let name = Ident::parse_any(s)?;

        if !crate::util::is_valid_tag(&name.to_string()) {
            return Err(Error::new(name.span(), "Not a valid Html tag"));
        }

        // parse the guts
        let content: ParseBuffer;
        syn::braced!(content in s);

        let mut attrs: Vec<Attr> = vec![];
        let mut children: Vec<Node> = vec![];
        parse_element_content(content, &mut attrs, &mut children)?;

        Ok(Self {
            name,
            attrs,
            children,
        })
    }
}

// used by both vcomponet and velement to parse the contents of the elements into attras and children
fn parse_element_content(
    stream: ParseBuffer,
    attrs: &mut Vec<Attr>,
    children: &mut Vec<Node>,
) -> Result<()> {
    'parsing: loop {
        // consume comma if it exists
        // we don't actually care if there *are* commas after elements/text
        if stream.peek(Token![,]) {
            let _ = stream.parse::<Token![,]>();
        }

        // [1] Break if empty
        if stream.is_empty() {
            break 'parsing;
        }

        // [2] Try to consume an attr
        let fork = stream.fork();
        if let Ok(attr) = fork.parse::<Attr>() {
            // make sure to advance or your computer will become a space heater :)
            stream.advance_to(&fork);
            attrs.push(attr);
            continue 'parsing;
        }

        // [3] Try to consume a child node
        let fork = stream.fork();
        if let Ok(node) = fork.parse::<Node>() {
            // make sure to advance or your computer will become a space heater :)
            stream.advance_to(&fork);
            children.push(node);
            continue 'parsing;
        }

        // todo: pass a descriptive error onto the offending tokens
        panic!("Entry is not an attr or element\n {}", stream)
    }
    Ok(())
}
fn try_parse_bracketed(stream: &ParseBuffer) -> Result<Expr> {
    let content;
    syn::braced!(content in stream);
    content.parse()
}

/// =======================================
/// Parse a VElement's Attributes
/// =======================================
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
                outer.parse()?
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

impl ToTokens for &Attr {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let name = self.name.to_string();
        let nameident = &self.name;
        let _attr_stream = TokenStream2::new();
        match &self.ty {
            AttrType::Value(value) => {
                tokens.append_all(quote! {
                    .attr(#name, #value)
                });
            }
            AttrType::Event(event) => {
                tokens.append_all(quote! {
                    .add_listener(dioxus::events::on::#nameident(ctx, #event))
                });
            }
            AttrType::Tok(exp) => {
                tokens.append_all(quote! {
                    .add_listener(dioxus::events::on::#nameident(ctx, #exp))
                });
            }
        }
    }
}

enum AttrType {
    Value(LitStr),
    Event(ExprClosure),
    Tok(Expr),
}

// =======================================
// Parse just plain text
// =======================================
struct TextNode(LitStr);

impl Parse for TextNode {
    fn parse(s: ParseStream) -> Result<Self> {
        Ok(Self(s.parse()?))
    }
}

impl ToTokens for TextNode {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        // todo: use heuristics to see if we can promote to &static str
        let token_stream = &self.0.to_token_stream();
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
