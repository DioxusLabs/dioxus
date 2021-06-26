use super::*;

use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens, TokenStreamExt};
use syn::{
    ext::IdentExt,
    parse::{discouraged::Speculative, Parse, ParseBuffer, ParseStream},
    token, Error, Expr, ExprClosure, Ident, LitStr, Result, Token,
};

// =======================================
// Parse the VNode::Element type
// =======================================
pub struct Element {
    name: Ident,
    attrs: Vec<ElementAttr>,
    children: Vec<Node>,
}

impl Parse for Element {
    fn parse(stream: ParseStream) -> Result<Self> {
        //
        let name = Ident::parse(stream)?;

        if !crate::util::is_valid_tag(&name.to_string()) {
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
                && forked.parse::<Token![:]>().is_err()
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

impl ToTokens for Element {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let name = &self.name.to_string();

        tokens.append_all(quote! {
            dioxus::builder::ElementBuilder::new(__cx, #name)
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
    BumpText(LitStr),
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
            match name_str.as_str() {
                "key" => {
                    // todo: better error here
                    AttrType::BumpText(s.parse::<LitStr>()?)
                }
                "style" => {
                    //
                    todo!("inline style not yet supported")
                }
                "classes" => {
                    //
                    todo!("custom class lsit not supported")
                }
                "namespace" => {
                    //
                    todo!("custom namespace not supported")
                }
                "ref" => {
                    //
                    todo!("custom ref not supported")
                }
                _ => {
                    if s.peek(LitStr) {
                        let rawtext = s.parse::<LitStr>().unwrap();
                        AttrType::BumpText(rawtext)
                    } else {
                        let toks = s.parse::<Expr>()?;
                        AttrType::FieldTokens(toks)
                    }
                }
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

impl ToTokens for ElementAttr {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let name = self.name.to_string();
        let nameident = &self.name;
        let _attr_stream = TokenStream2::new();

        match &self.ty {
            AttrType::BumpText(value) => match name.as_str() {
                "key" => {
                    tokens.append_all(quote! {
                        .key2(format_args_f!(#value))
                    });
                }
                _ => {
                    tokens.append_all(quote! {
                        .attr(#name, format_args_f!(#value))
                    });
                }
            },
            AttrType::FieldTokens(exp) => {
                tokens.append_all(quote! {
                    .attr(#name, #exp)
                });
            }
            AttrType::Event(event) => {
                tokens.append_all(quote! {
                    .add_listener(dioxus::events::on::#nameident(__cx, #event))
                });
            }
            AttrType::EventTokens(event) => {
                //
                tokens.append_all(quote! {
                    .add_listener(dioxus::events::on::#nameident(__cx, #event))
                });
            }
        }
    }
}
