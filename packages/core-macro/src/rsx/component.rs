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
    token, Error, Expr, ExprClosure, Ident, Result, Token,
};

pub struct Component<const AS: HTML_OR_RSX> {
    // accept any path-like argument
    name: syn::Path,
    body: Vec<ComponentField<AS>>,
    children: Vec<BodyNode<AS>>,
    manual_props: Option<Expr>,
}

impl Parse for Component<AS_RSX> {
    fn parse(stream: ParseStream) -> Result<Self> {
        // let name = s.parse::<syn::ExprPath>()?;
        // todo: look into somehow getting the crate/super/etc

        let name = syn::Path::parse_mod_style(stream)?;

        // parse the guts
        let content: ParseBuffer;
        syn::braced!(content in stream);

        let mut body: Vec<ComponentField<AS_RSX>> = Vec::new();
        let mut children: Vec<BodyNode<AS_RSX>> = Vec::new();
        let mut manual_props = None;

        let cfg: BodyParseConfig<AS_RSX> = BodyParseConfig {
            allow_children: true,
            allow_fields: true,
            allow_manual_props: true,
        };

        cfg.parse_component_body(&content, &mut body, &mut children, &mut manual_props)?;

        Ok(Self {
            name,
            body,
            children,
            manual_props,
        })
    }
}
impl Parse for Component<AS_HTML> {
    fn parse(stream: ParseStream) -> Result<Self> {
        let name = syn::Path::parse_mod_style(stream)?;

        // parse the guts
        let content: ParseBuffer;
        syn::braced!(content in stream);

        let mut body: Vec<ComponentField<AS_HTML>> = Vec::new();
        let mut children: Vec<BodyNode<AS_HTML>> = Vec::new();
        let mut manual_props = None;

        let cfg: BodyParseConfig<AS_HTML> = BodyParseConfig {
            allow_children: true,
            allow_fields: true,
            allow_manual_props: true,
        };

        cfg.parse_component_body(&content, &mut body, &mut children, &mut manual_props)?;

        Ok(Self {
            name,
            body,
            children,
            manual_props,
        })
    }
}

pub struct BodyParseConfig<const AS: HTML_OR_RSX> {
    pub allow_fields: bool,
    pub allow_children: bool,
    pub allow_manual_props: bool,
}
impl BodyParseConfig<AS_RSX> {
    // todo: unify this body parsing for both elements and components
    // both are style rather ad-hoc, though components are currently more configured
    pub fn parse_component_body(
        &self,
        content: &ParseBuffer,
        body: &mut Vec<ComponentField<AS_RSX>>,
        children: &mut Vec<BodyNode<AS_RSX>>,
        manual_props: &mut Option<Expr>,
    ) -> Result<()> {
        'parsing: loop {
            // [1] Break if empty
            if content.is_empty() {
                break 'parsing;
            }

            if content.peek(Token![..]) {
                if !self.allow_manual_props {
                    return Err(Error::new(
                        content.span(),
                        "Props spread syntax is not allowed in this context. \nMake to only use the elipsis `..` in Components.",
                    ));
                }
                content.parse::<Token![..]>()?;
                *manual_props = Some(content.parse::<Expr>()?);
            } else if content.peek(Ident) && content.peek2(Token![:]) && !content.peek3(Token![:]) {
                if !self.allow_fields {
                    return Err(Error::new(
                        content.span(),
                        "Property fields is not allowed in this context. \nMake to only use fields in Components or Elements.",
                    ));
                }
                body.push(content.parse::<ComponentField<AS_RSX>>()?);
            } else {
                if !self.allow_children {
                    return Err(Error::new(
                        content.span(),
                        "This item is not allowed to accept children.",
                    ));
                }
                children.push(content.parse::<BodyNode<AS_RSX>>()?);
            }

            // consume comma if it exists
            // we don't actually care if there *are* commas between attrs
            if content.peek(Token![,]) {
                let _ = content.parse::<Token![,]>();
            }
        }
        Ok(())
    }
}
impl BodyParseConfig<AS_HTML> {
    // todo: unify this body parsing for both elements and components
    // both are style rather ad-hoc, though components are currently more configured
    pub fn parse_component_body(
        &self,
        content: &ParseBuffer,
        body: &mut Vec<ComponentField<AS_HTML>>,
        children: &mut Vec<BodyNode<AS_HTML>>,
        manual_props: &mut Option<Expr>,
    ) -> Result<()> {
        'parsing: loop {
            // [1] Break if empty
            if content.is_empty() {
                break 'parsing;
            }

            if content.peek(Token![..]) {
                if !self.allow_manual_props {
                    return Err(Error::new(
                        content.span(),
                        "Props spread syntax is not allowed in this context. \nMake to only use the elipsis `..` in Components.",
                    ));
                }
                content.parse::<Token![..]>()?;
                *manual_props = Some(content.parse::<Expr>()?);
            } else if content.peek(Ident) && content.peek2(Token![:]) && !content.peek3(Token![:]) {
                if !self.allow_fields {
                    return Err(Error::new(
                        content.span(),
                        "Property fields is not allowed in this context. \nMake to only use fields in Components or Elements.",
                    ));
                }
                body.push(content.parse::<ComponentField<AS_HTML>>()?);
            } else {
                if !self.allow_children {
                    return Err(Error::new(
                        content.span(),
                        "This item is not allowed to accept children.",
                    ));
                }
                children.push(content.parse::<BodyNode<AS_HTML>>()?);
            }

            // consume comma if it exists
            // we don't actually care if there *are* commas between attrs
            if content.peek(Token![,]) {
                let _ = content.parse::<Token![,]>();
            }
        }
        Ok(())
    }
}

impl<const AS: HTML_OR_RSX> ToTokens for Component<AS> {
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
            __cx.component(
                #name,
                #builder,
                #key_token,
                __cx.bump().alloc(#children)
            )
        })
    }
}

// the struct's fields info
pub struct ComponentField<const AS: HTML_OR_RSX> {
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

impl Parse for ComponentField<AS_RSX> {
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
impl Parse for ComponentField<AS_HTML> {
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

impl<const AS: HTML_OR_RSX> ToTokens for ComponentField<AS> {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let ComponentField { name, content, .. } = self;
        tokens.append_all(quote! {
            .#name(#content)
        })
    }
}
