use syn::parse::{discouraged::Speculative, ParseBuffer};

use crate::util::is_valid_html_tag;

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
    root: AmbiguousElement,
}

impl Parse for RsxRender {
    fn parse(input: ParseStream) -> Result<Self> {
        // try to parse the first ident and comma
        let custom_context =
            if input.peek(Token![in]) && input.peek2(Ident) && input.peek3(Token![,]) {
                let _ = input.parse::<Token![in]>()?;
                let name = input.parse::<Ident>()?;
                if is_valid_html_tag(&name.to_string()) {
                    return Err(Error::new(
                        input.span(),
                        "Custom context cannot be an html element name",
                    ));
                } else {
                    input.parse::<Token![,]>().unwrap();
                    Some(name)
                }
            } else {
                None
            };

        let root = { input.parse::<AmbiguousElement>() }?;
        if !input.is_empty() {
            return Err(Error::new(
                input.span(),
                "Currently only one element is allowed per component",
            ));
        }

        Ok(Self {
            root,
            custom_context,
        })
    }
}

impl ToTokens for RsxRender {
    fn to_tokens(&self, out_tokens: &mut TokenStream2) {
        // create a lazy tree that accepts a bump allocator
        // Currently disabled

        let inner = &self.root;

        let output = match &self.custom_context {
            Some(ident) => {
                //
                quote! {
                    #ident.render(dioxus::prelude::LazyNodes::new(move |__ctx|{
                        let bump = &__ctx.bump();
                        #inner
                    }))
                }
            }
            None => {
                quote! {
                    dioxus::prelude::LazyNodes::new(move |__ctx|{
                        let bump = &__ctx.bump();
                        #inner
                     })
                }
            }
        };

        output.to_tokens(out_tokens)
    }
}

enum AmbiguousElement {
    Element(Element),
    Component(Component),
}

impl Parse for AmbiguousElement {
    fn parse(input: ParseStream) -> Result<Self> {
        // Try to parse as an absolute path and immediately defer to the componetn
        if input.peek(Token![::]) {
            return input
                .parse::<Component>()
                .map(|c| AmbiguousElement::Component(c));
        }

        // If not an absolute path, then parse the ident and check if it's a valid tag

        if let Ok(pat) = input.fork().parse::<syn::Path>() {
            if pat.segments.len() > 1 {
                return input
                    .parse::<Component>()
                    .map(|c| AmbiguousElement::Component(c));
            }
        }

        if let Ok(name) = input.fork().parse::<Ident>() {
            let name_str = name.to_string();

            match is_valid_html_tag(&name_str) {
                true => input
                    .parse::<Element>()
                    .map(|c| AmbiguousElement::Element(c)),
                false => {
                    let first_char = name_str.chars().next().unwrap();
                    if first_char.is_ascii_uppercase() {
                        input
                            .parse::<Component>()
                            .map(|c| AmbiguousElement::Component(c))
                    } else {
                        let name = input.parse::<Ident>().unwrap();
                        Err(Error::new(
                            name.span(),
                            "Components must be uppercased, perhaps you mispelled a html tag",
                        ))
                    }
                }
            }
        } else {
            Err(Error::new(input.span(), "Not a valid Html tag"))
        }
    }
}

impl ToTokens for AmbiguousElement {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        match self {
            AmbiguousElement::Element(el) => el.to_tokens(tokens),
            AmbiguousElement::Component(comp) => comp.to_tokens(tokens),
        }
    }
}

// ==============================================
// Parse any div {} as a VElement
// ==============================================
enum Node {
    Element(AmbiguousElement),
    Text(TextNode),
    RawExpr(Expr),
}

impl ToTokens for &Node {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        match &self {
            Node::Element(el) => el.to_tokens(tokens),
            Node::Text(txt) => txt.to_tokens(tokens),
            Node::RawExpr(exp) => exp.to_tokens(tokens),
        }
    }
}

impl Parse for Node {
    fn parse(stream: ParseStream) -> Result<Self> {
        // Supposedly this approach is discouraged due to inability to return proper errors
        // TODO: Rework this to provide more informative errors

        if stream.peek(token::Brace) {
            let content;
            syn::braced!(content in stream);
            return Ok(Node::RawExpr(content.parse::<Expr>()?));
        }

        if stream.peek(LitStr) {
            return Ok(Node::Text(stream.parse::<TextNode>()?));
        }

        Ok(Node::Element(stream.parse::<AmbiguousElement>()?))
    }
}

struct Component {
    // accept any path-like argument
    name: syn::Path,
    body: Vec<ComponentField>,
    children: Vec<Node>,
}

impl Parse for Component {
    fn parse(s: ParseStream) -> Result<Self> {
        // let name = s.parse::<syn::ExprPath>()?;
        // todo: look into somehow getting the crate/super/etc

        let name = syn::Path::parse_mod_style(s)?;

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

            body.push(content.parse::<ComponentField>()?);

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

impl ToTokens for &Component {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let name = &self.name;

        let mut builder = quote! {
            fc_to_builder(#name)
        };

        let mut has_key = None;

        for field in &self.body {
            if field.name.to_string() == "key" {
                has_key = Some(field);
            } else {
                builder.append_all(quote! {#field});
            }
        }

        builder.append_all(quote! {
            .build()
        });

        let key_token = match has_key {
            Some(field) => {
                let inners = field.content.to_token_stream();
                quote! {
                    Some(#inners)
                }
            }
            None => quote! {None},
        };

        let _toks = tokens.append_all(quote! {
            dioxus::builder::virtual_child(__ctx, #name, #builder, #key_token)
        });
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
    attrs: Vec<ElementAttr>,
    children: Vec<Node>,
}

impl Parse for Element {
    fn parse(stream: ParseStream) -> Result<Self> {
        //
        let name = Ident::parse(stream)?;

        if !crate::util::is_valid_html_tag(&name.to_string()) {
            return Err(Error::new(name.span(), "Not a valid Html tag"));
        }

        // parse the guts
        let content: ParseBuffer;
        syn::braced!(content in stream);

        let mut attrs: Vec<ElementAttr> = vec![];
        let mut children: Vec<Node> = vec![];
        'parsing: loop {
            // [1] Break if empty
            if content.is_empty() {
                break 'parsing;
            }

            let forked = content.fork();
            if forked.call(Ident::parse_any).is_ok()
                && forked.parse::<Token![:]>().is_ok()
                && forked.parse::<Expr>().is_ok()
            {
                attrs.push(content.parse::<ElementAttr>()?);
            } else {
                children.push(content.parse::<Node>()?);
            }

            // consume comma if it exists
            // we don't actually care if there *are* commas after elements/text
            if content.peek(Token![,]) {
                let _ = content.parse::<Token![,]>();
            }
        }

        Ok(Self {
            name,
            attrs,
            children,
        })
    }
}

impl ToTokens for &Element {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let name = &self.name.to_string();

        tokens.append_all(quote! {
            dioxus::builder::ElementBuilder::new(__ctx, #name)
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

/// =======================================
/// Parse a VElement's Attributes
/// =======================================
struct ElementAttr {
    name: Ident,
    ty: AttrType,
}

enum AttrType {
    Value(LitStr),
    FieldTokens(Expr),
    EventTokens(Expr),
    Event(ExprClosure),
}

impl Parse for ElementAttr {
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
                    AttrType::EventTokens(content.parse()?)
                }
            } else {
                AttrType::Event(s.parse()?)
            }
        } else {
            let fork = s.fork();
            if let Ok(rawtext) = fork.parse::<LitStr>() {
                s.advance_to(&fork);
                AttrType::Value(rawtext)
            } else {
                let toks = s.parse::<Expr>()?;
                AttrType::FieldTokens(toks)
            }
            // let lit_str = if name_str == "style" && s.peek(token::Brace) {
            //     // special-case to deal with literal styles.
            //     let outer;
            //     syn::braced!(outer in s);
            //     // double brace for inline style.
            //     // todo!("Style support not ready yet");

            //     // if outer.peek(token::Brace) {
            //     //     let inner;
            //     //     syn::braced!(inner in outer);
            //     //     let styles: Styles = inner.parse()?;
            //     //     MaybeExpr::Literal(LitStr::new(&styles.to_string(), Span::call_site()))
            //     // } else {
            //     // just parse as an expression
            //     outer.parse()?
            // // }
            // } else {
            //     s.parse()?
            // };
        };

        // consume comma if it exists
        // we don't actually care if there *are* commas between attrs
        if s.peek(Token![,]) {
            let _ = s.parse::<Token![,]>();
        }

        Ok(ElementAttr { name, ty })
    }
}

impl ToTokens for &ElementAttr {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let name = self.name.to_string();
        let nameident = &self.name;
        let _attr_stream = TokenStream2::new();
        match &self.ty {
            AttrType::Value(value) => {
                tokens.append_all(quote! {
                    .attr(#name, {
                        use bumpalo::core_alloc::fmt::Write;
                        let mut s = bumpalo::collections::String::new_in(bump);
                        s.write_fmt(format_args_f!(#value)).unwrap();
                        s.into_bump_str()
                    })
                });
            }
            AttrType::Event(event) => {
                tokens.append_all(quote! {
                    .add_listener(dioxus::events::on::#nameident(__ctx, #event))
                });
            }
            AttrType::FieldTokens(exp) => {
                tokens.append_all(quote! {
                    .attr(#name, #exp)
                });
            }
            AttrType::EventTokens(event) => {
                //
                tokens.append_all(quote! {
                    .add_listener(dioxus::events::on::#nameident(__ctx, #event))
                })
            }
        }
    }
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

fn try_parse_bracketed(stream: &ParseBuffer) -> Result<Expr> {
    let content;
    syn::braced!(content in stream);
    content.parse()
}

// // Used to uniquely identify elements that contain closures so that the DomUpdater can
// // look them up by their unique id.
// // When the DomUpdater sees that the element no longer exists it will drop all of it's
// // Rc'd Closures for those events.
// // It doesn't quite make sense to keep this here, perhaps just in the html crate?
// // Dioxus itself shouldn't be concerned with the attribute names
// // a ftk!
// static SELF_CLOSING_TAGS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
//     [
//         "area", "base", "br", "col", "hr", "img", "input", "link", "meta", "param", "command",
//         "keygen", "source",
//     ]
//     .iter()
//     .cloned()
//     .collect()
// });

// /// Whether or not this tag is self closing
// ///
// /// ```ignore
// /// use dioxus_core::validation::is_self_closing;
// /// assert_eq!(is_self_closing("br"), true);
// /// assert_eq!(is_self_closing("div"), false);
// /// ```
// pub fn is_self_closing(tag: &str) -> bool {
//     SELF_CLOSING_TAGS.contains(tag)
//     // SELF_CLOSING_TAGS.contains(tag) || is_self_closing_svg_tag(tag)
// }
