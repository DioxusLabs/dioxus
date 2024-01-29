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
    spanned::Spanned,
    token::Brace,
    AngleBracketedGenericArguments, Error, Expr, Ident, LitStr, PathArguments, Result, Token,
};

#[derive(PartialEq, Eq, Clone, Debug, Hash)]
pub struct Component {
    pub name: syn::Path,
    pub prop_gen_args: Option<AngleBracketedGenericArguments>,
    pub fields: Vec<ComponentField>,
    pub children: Vec<BodyNode>,
    pub manual_props: Option<Expr>,
    pub brace: syn::token::Brace,
}

impl Parse for Component {
    fn parse(stream: ParseStream) -> Result<Self> {
        let mut name = stream.parse::<syn::Path>()?;
        Component::validate_component_path(&name)?;

        // extract the path arguments from the path into prop_gen_args
        let prop_gen_args = normalize_path(&mut name);

        let content: ParseBuffer;
        let brace = syn::braced!(content in stream);

        let mut fields = Vec::new();
        let mut children = Vec::new();
        let mut manual_props = None;

        while !content.is_empty() {
            // if we splat into a component then we're merging properties
            if content.peek(Token![..]) {
                content.parse::<Token![..]>()?;
                manual_props = Some(content.parse()?);
            } else if
            // Named fields
            (content.peek(Ident) && content.peek2(Token![:]) && !content.peek3(Token![:]))
            // shorthand struct initialization
                // Not a div {}, mod::Component {}, or web-component {}
                || (content.peek(Ident)
                    && !content.peek2(Brace)
                    && !content.peek2(Token![:])
                    && !content.peek2(Token![-]))
            {
                fields.push(content.parse()?);
            } else {
                children.push(content.parse()?);
            }

            if content.peek(Token![,]) {
                let _ = content.parse::<Token![,]>();
            }
        }

        Ok(Self {
            name,
            prop_gen_args,
            fields,
            children,
            manual_props,
            brace,
        })
    }
}

impl ToTokens for Component {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let Self {
            name,
            prop_gen_args,
            ..
        } = self;

        let builder = self
            .manual_props
            .as_ref()
            .map(|props| self.collect_manual_props(props))
            .unwrap_or_else(|| self.collect_props());

        let fn_name = self.fn_name();

        tokens.append_all(quote! {
            dioxus_core::DynamicNode::Component({
                use dioxus_core::prelude::Properties;
                (#builder).into_vcomponent(
                    #name #prop_gen_args,
                    #fn_name
                )
            })
        })
    }
}

impl Component {
    fn validate_component_path(path: &syn::Path) -> Result<()> {
        // ensure path segments doesn't have PathArguments, only the last
        // segment is allowed to have one.
        if path
            .segments
            .iter()
            .take(path.segments.len() - 1)
            .any(|seg| seg.arguments != PathArguments::None)
        {
            component_path_cannot_have_arguments!(path.span());
        }

        // ensure last segment only have value of None or AngleBracketed
        if !matches!(
            path.segments.last().unwrap().arguments,
            PathArguments::None | PathArguments::AngleBracketed(_)
        ) {
            invalid_component_path!(path.span());
        }

        Ok(())
    }

    pub fn key(&self) -> Option<&IfmtInput> {
        match self
            .fields
            .iter()
            .find(|f| f.name == "key")
            .map(|f| &f.content)
        {
            Some(ContentField::Formatted(fmt)) => Some(fmt),
            _ => None,
        }
    }

    fn collect_manual_props(&self, manual_props: &Expr) -> TokenStream2 {
        let mut toks = quote! { let mut __manual_props = #manual_props; };
        for field in &self.fields {
            if field.name == "key" {
                continue;
            }
            let ComponentField { name, content } = field;
            toks.append_all(quote! { __manual_props.#name = #content; });
        }
        toks.append_all(quote! { __manual_props });
        quote! {{ #toks }}
    }

    fn collect_props(&self) -> TokenStream2 {
        let name = &self.name;

        let mut toks = match &self.prop_gen_args {
            Some(gen_args) => quote! { fc_to_builder(#name #gen_args) },
            None => quote! { fc_to_builder(#name) },
        };
        for field in &self.fields {
            match field.name.to_string().as_str() {
                "key" => {}
                _ => toks.append_all(quote! {#field}),
            }
        }
        if !self.children.is_empty() {
            let renderer: TemplateRenderer = TemplateRenderer {
                roots: &self.children,
                location: None,
            };

            toks.append_all(quote! {
                .children(
                    Some({ #renderer })
                )
            });
        }
        toks.append_all(quote! {
            .build()
        });
        toks
    }

    fn fn_name(&self) -> String {
        self.name.segments.last().unwrap().ident.to_string()
    }
}

// the struct's fields info
#[derive(PartialEq, Eq, Clone, Debug, Hash)]
pub struct ComponentField {
    pub name: Ident,
    pub content: ContentField,
}

#[derive(PartialEq, Eq, Clone, Debug, Hash)]
pub enum ContentField {
    Shorthand(Ident),
    ManExpr(Expr),
    Formatted(IfmtInput),
    OnHandlerRaw(Expr),
}

impl ContentField {
    fn new_from_name(name: &Ident, input: ParseStream) -> Result<Self> {
        if name.to_string().starts_with("on") {
            return Ok(ContentField::OnHandlerRaw(input.parse()?));
        }

        if *name == "key" {
            return Ok(ContentField::Formatted(input.parse()?));
        }

        if input.peek(LitStr) {
            let forked = input.fork();
            let t: LitStr = forked.parse()?;

            // the string literal must either be the end of the input or a followed by a comma
            let res =
                match (forked.is_empty() || forked.peek(Token![,])) && is_literal_foramtted(&t) {
                    true => ContentField::Formatted(input.parse()?),
                    false => ContentField::ManExpr(input.parse()?),
                };

            return Ok(res);
        }

        Ok(ContentField::ManExpr(input.parse()?))
    }
}

impl ToTokens for ContentField {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        match self {
            ContentField::Shorthand(i) if i.to_string().starts_with("on") => {
                tokens.append_all(quote! { EventHandler::new(#i) })
            }
            ContentField::Shorthand(i) => tokens.append_all(quote! { #i }),
            ContentField::ManExpr(e) => e.to_tokens(tokens),
            ContentField::Formatted(s) => tokens.append_all(quote! {
                #s
            }),
            ContentField::OnHandlerRaw(e) => tokens.append_all(quote! {
                EventHandler::new(#e)
            }),
        }
    }
}

impl Parse for ComponentField {
    fn parse(input: ParseStream) -> Result<Self> {
        let name = Ident::parse_any(input)?;

        // if the next token is not a colon, then it's a shorthand field
        if input.parse::<Token![:]>().is_err() {
            return Ok(Self {
                content: ContentField::Shorthand(name.clone()),
                name,
            });
        };

        let content = ContentField::new_from_name(&name, input)?;

        if input.peek(LitStr) || input.peek(Ident) {
            missing_trailing_comma!(content.span());
        }

        Ok(Self { name, content })
    }
}

impl ToTokens for ComponentField {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let ComponentField { name, content, .. } = self;
        tokens.append_all(quote! { .#name(#content) })
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

fn normalize_path(name: &mut syn::Path) -> Option<AngleBracketedGenericArguments> {
    let seg = name.segments.last_mut()?;
    match seg.arguments.clone() {
        PathArguments::AngleBracketed(args) => {
            seg.arguments = PathArguments::None;
            Some(args)
        }
        _ => None,
    }
}
