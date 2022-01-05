//! Parse components into the VComponent VNode
//! ==========================================
//!
//! This parsing path emerges from [`AmbiguousElement`] which supports validation of the vcomponent format.
//! We can be reasonably sure that whatever enters this parsing path is in the right format.
//! This feature must support
//! - [x] Namespaced components
//! - [x] Fields
//! - [x] Componentbuilder synax
//! - [x] Optional commas
//! - [ ] Children
//! - [ ] Keys
//! - [ ] Properties spreading with with `..` syntax

use super::*;

use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens, TokenStreamExt};
use syn::{
    ext::IdentExt,
    parse::{Parse, ParseBuffer, ParseStream},
    token, Expr, Ident, LitStr, Result, Token,
};

pub struct Component {
    name: syn::Path,
    body: Vec<ComponentField>,
    children: Vec<BodyNode>,
    manual_props: Option<Expr>,
}

impl Parse for Component {
    fn parse(stream: ParseStream) -> Result<Self> {
        let name = syn::Path::parse_mod_style(stream)?;

        let content: ParseBuffer;

        // if we see a `{` then we have a block
        // else parse as a function-like call
        if stream.peek(token::Brace) {
            syn::braced!(content in stream);
        } else {
            syn::parenthesized!(content in stream);
        }

        let mut body = Vec::new();
        let mut children = Vec::new();
        let mut manual_props = None;

        while !content.is_empty() {
            // if we splat into a component then we're merging properties
            if content.peek(Token![..]) {
                content.parse::<Token![..]>()?;
                manual_props = Some(content.parse::<Expr>()?);
            } else if content.peek(Ident) && content.peek2(Token![:]) && !content.peek3(Token![:]) {
                body.push(content.parse::<ComponentField>()?);
            } else {
                children.push(content.parse::<BodyNode>()?);
            }

            if content.peek(Token![,]) {
                let _ = content.parse::<Token![,]>();
            }
        }

        Ok(Self {
            name,
            body,
            children,
            manual_props,
        })
    }
}

impl ToTokens for Component {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let name = &self.name;

        let mut has_key = None;

        let builder = match &self.manual_props {
            Some(manual_props) => {
                let mut toks = quote! {
                    let mut __manual_props = #manual_props;
                };
                for field in &self.body {
                    if field.name == "key" {
                        has_key = Some(field);
                    } else {
                        let name = &field.name;
                        let val = &field.content;
                        toks.append_all(quote! {
                            __manual_props.#name = #val;
                        });
                    }
                }
                toks.append_all(quote! {
                    __manual_props
                });
                quote! {{
                    #toks
                }}
            }
            None => {
                let mut toks = quote! { fc_to_builder(#name) };
                for field in &self.body {
                    match field.name.to_string().as_str() {
                        "key" => {
                            //
                            has_key = Some(field);
                        }
                        _ => toks.append_all(quote! {#field}),
                    }
                }

                if !self.children.is_empty() {
                    let childs = &self.children;
                    toks.append_all(quote! {
                        .children(__cx.create_children([ #( #childs ),* ]))
                    });
                }

                toks.append_all(quote! {
                    .build()
                });
                toks
            }
        };

        let key_token = match has_key {
            Some(field) => {
                let inners = &field.content;
                quote! { Some(format_args_f!(#inners)) }
            }
            None => quote! { None },
        };

        tokens.append_all(quote! {
            __cx.component(
                #name,
                #builder,
                #key_token,
            )
        })
    }
}

// the struct's fields info
pub struct ComponentField {
    name: Ident,
    content: ContentField,
}

enum ContentField {
    ManExpr(Expr),
    Formatted(LitStr),
    OnHandlerRaw(Expr),
}

impl ToTokens for ContentField {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        match self {
            ContentField::ManExpr(e) => e.to_tokens(tokens),
            ContentField::Formatted(s) => tokens.append_all(quote! {
                __cx.raw_text(format_args_f!(#s)).0
            }),
            ContentField::OnHandlerRaw(e) => tokens.append_all(quote! {
                __cx.bump().alloc(#e)
            }),
        }
    }
}

impl Parse for ComponentField {
    fn parse(input: ParseStream) -> Result<Self> {
        let name = Ident::parse_any(input)?;
        input.parse::<Token![:]>()?;

        if name.to_string().starts_with("on") {
            let content = ContentField::OnHandlerRaw(input.parse()?);
            return Ok(Self { name, content });
        }

        if name.to_string() == "key" {
            let content = ContentField::ManExpr(input.parse()?);
            return Ok(Self { name, content });
        }

        if input.peek(LitStr) && input.peek2(Token![,]) {
            let t: LitStr = input.fork().parse()?;

            if is_literal_foramtted(&t) {
                let content = ContentField::Formatted(input.parse()?);
                return Ok(Self { name, content });
            }
        }

        if input.peek(LitStr) && input.peek2(LitStr) {
            let item = input.parse::<LitStr>().unwrap();
            proc_macro_error::emit_error!(item, "This attribute is misisng a trailing comma")
        }

        let content = ContentField::ManExpr(input.parse()?);
        Ok(Self { name, content })
    }
}

impl ToTokens for ComponentField {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let ComponentField { name, content, .. } = self;
        tokens.append_all(quote! {
            .#name(#content)
        })
    }
}

fn is_literal_foramtted(lit: &LitStr) -> bool {
    let s = lit.value();
    let mut chars = s.chars();

    while let Some(next) = chars.next() {
        if next == '{' {
            let nen = chars.next();
            if nen == Some('{') {
                return true;
            }
        }
    }

    false
}
