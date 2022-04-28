//! Parse components into the VNode::Component variant
//! ==========================================
//!
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
    token, AngleBracketedGenericArguments, Expr, Ident, LitStr, PathArguments, Result, Token,
};

pub struct Component {
    pub name: syn::Path,
    pub prop_gen_args: Option<AngleBracketedGenericArguments>,
    pub body: Vec<ComponentField>,
    pub children: Vec<BodyNode>,
    pub manual_props: Option<Expr>,
}

impl Component {
    pub fn validate_component_path(path: &syn::Path) -> Result<()> {
        // ensure path segments doesn't have PathArguments, only the last
        // segment is allowed to have one.
        if path
            .segments
            .iter()
            .take(path.segments.len() - 1)
            .any(|seg| seg.arguments != PathArguments::None)
        {
            component_path_cannot_have_arguments!(path);
        }

        // ensure last segment only have value of None or AngleBracketed
        if !matches!(
            path.segments.last().unwrap().arguments,
            PathArguments::None | PathArguments::AngleBracketed(_)
        ) {
            invalid_component_path!(path);
        }

        // if matches!(
        //     path.segments.last().unwrap().arguments,
        //     PathArguments::AngleBracketed(_)
        // ) {
        //     proc_macro_error::abort!(path, "path: {}", path.to_token_stream().to_string());
        // }

        Ok(())
    }
}

impl Parse for Component {
    fn parse(stream: ParseStream) -> Result<Self> {
        let mut name = stream.parse::<syn::Path>()?;
        Component::validate_component_path(&name)?;

        // extract the path arguments from the path into prop_gen_args
        let prop_gen_args = name.segments.last_mut().and_then(|seg| {
            if let PathArguments::AngleBracketed(args) = seg.arguments.clone() {
                seg.arguments = PathArguments::None;
                Some(args)
            } else {
                None
            }
        });

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
            prop_gen_args,
            body,
            children,
            manual_props,
        })
    }
}

impl ToTokens for Component {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let name = &self.name;
        let prop_gen_args = &self.prop_gen_args;

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
                let mut toks = match prop_gen_args {
                    Some(gen_args) => quote! { fc_to_builder::#gen_args(#name) },
                    None => quote! { fc_to_builder(#name) },
                };
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

        let fn_name = self.name.segments.last().unwrap().ident.to_string();

        tokens.append_all(quote! {
            __cx.component(
                #name,
                #builder,
                #key_token,
                #fn_name
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
                __cx.event_handler(#e)
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

        if name == "key" {
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
            missing_trailing_comma!(input.span());
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
            if nen != Some('{') {
                return true;
            }
        }
    }

    false
}
