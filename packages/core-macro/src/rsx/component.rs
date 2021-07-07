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
    token, Expr, ExprClosure, Ident, Result, Token,
};

pub struct Component {
    // accept any path-like argument
    name: syn::Path,
    body: Vec<ComponentField>,
    children: Vec<Node>,
    manual_props: Option<Expr>,
}

impl Parse for Component {
    fn parse(stream: ParseStream) -> Result<Self> {
        // let name = s.parse::<syn::ExprPath>()?;
        // todo: look into somehow getting the crate/super/etc

        let name = syn::Path::parse_mod_style(stream)?;

        // parse the guts
        let content: ParseBuffer;
        syn::braced!(content in stream);

        let mut body: Vec<ComponentField> = Vec::new();
        let mut children: Vec<Node> = Vec::new();
        let mut manual_props = None;

        parse_component_body(
            &content,
            &BodyParseConfig {
                allow_children: true,
                allow_fields: true,
                allow_manual_props: true,
            },
            &mut body,
            &mut children,
            &mut manual_props,
        )?;

        Ok(Self {
            name,
            body,
            children,
            manual_props,
        })
    }
}

pub struct BodyParseConfig {
    pub allow_fields: bool,
    pub allow_children: bool,
    pub allow_manual_props: bool,
}

pub fn parse_component_body(
    content: &ParseBuffer,
    cfg: &BodyParseConfig,
    body: &mut Vec<ComponentField>,
    children: &mut Vec<Node>,
    manual_props: &mut Option<Expr>,
) -> Result<()> {
    'parsing: loop {
        // [1] Break if empty
        if content.is_empty() {
            break 'parsing;
        }

        if content.peek(Token![..]) {
            if !cfg.allow_manual_props {
                // toss an error
            }
            content.parse::<Token![..]>()?;
            *manual_props = Some(content.parse::<Expr>()?);
        } else if content.peek(Ident) && content.peek2(Token![:]) {
            if !cfg.allow_fields {
                // toss an error
            }
            body.push(content.parse::<ComponentField>()?);
        } else {
            if !cfg.allow_children {
                // toss an error
            }
            children.push(content.parse::<Node>()?);
        }

        // consume comma if it exists
        // we don't actually care if there *are* commas between attrs
        if content.peek(Token![,]) {
            let _ = content.parse::<Token![,]>();
        }
    }
    Ok(())
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
                    if field.name.to_string() == "key" {
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
                    if field.name.to_string() == "key" {
                        has_key = Some(field);
                    } else {
                        toks.append_all(quote! {#field})
                    }
                }
                toks.append_all(quote! {
                    .build()
                });
                toks
            }
        };

        let key_token = match has_key {
            Some(field) => {
                let inners = field.content.to_token_stream();
                quote! {
                    Some(#inners)
                }
            }
            None => quote! {None},
        };

        let childs = &self.children;
        let children = quote! {
            [ #( #childs ),* ]
        };

        tokens.append_all(quote! {
            __cx.virtual_child(
                #name,
                #builder,
                #key_token,
                #children
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
    OnHandler(ExprClosure),

    // A handler was provided in {} tokens
    OnHandlerRaw(Expr),
}

impl ToTokens for ContentField {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        match self {
            ContentField::ManExpr(e) => e.to_tokens(tokens),
            ContentField::OnHandler(e) => tokens.append_all(quote! {
                __cx.bump().alloc(#e)
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

        let name_str = name.to_string();
        let content = if name_str.starts_with("on") {
            if input.peek(token::Brace) {
                let content;
                syn::braced!(content in input);
                ContentField::OnHandlerRaw(content.parse()?)
            } else {
                ContentField::OnHandler(input.parse()?)
            }
        } else {
            ContentField::ManExpr(input.parse::<Expr>()?)
        };

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
