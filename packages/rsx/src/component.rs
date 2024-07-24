//! Parse components into the VNode::Component variant
//!
//! Uses the regular robust RsxBlock parser and then validates the component, emitting errors as
//! diagnostics. This was refactored from a straightforward parser to this validation approach so
//! that we can emit errors as diagnostics instead of returning results.
//!
//! Using this approach we can provide *much* better errors as well as partial expansion wherever
//! possible.
//!
//! It does lead to the code actually being larger than it was before, but it should be much easier
//! to work with and extend. To add new syntax, we add it to the RsxBlock parser and then add a
//! validation step here. This does make using the component as a source of truth not as good, but
//! oddly enoughly, we want the tree to actually be capable of being technically invalid. This is not
//! usual for building in Rust - you want strongly typed things to be valid - but in this case, we
//! want to accept all sorts of malformed input and then provide the best possible error messages.
//!
//! If you're generally parsing things, you'll just want to parse and then check if it's valid.

use crate::innerlude::*;
use proc_macro2::TokenStream as TokenStream2;
use proc_macro2_diagnostics::SpanDiagnosticExt;
use quote::{quote, ToTokens, TokenStreamExt};
use std::collections::HashSet;
use syn::{
    parse::{Parse, ParseStream},
    spanned::Spanned,
    token, AngleBracketedGenericArguments, Expr, Ident, PathArguments, Result,
};

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct Component {
    pub name: syn::Path,
    pub generics: Option<AngleBracketedGenericArguments>,
    pub fields: Vec<Attribute>,
    pub spreads: Vec<Spread>,
    pub brace: token::Brace,
    pub children: TemplateBody,
    pub dyn_idx: DynIdx,
    pub diagnostics: Diagnostics,
}

impl Parse for Component {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut name = input.parse::<syn::Path>()?;
        let generics = normalize_path(&mut name);

        if !input.peek(token::Brace) {
            return Ok(Self::empty(name, generics));
        };

        let RsxBlock {
            attributes: fields,
            children,
            brace,
            spreads,
            diagnostics,
        } = input.parse::<RsxBlock>()?;

        let mut component = Self {
            dyn_idx: DynIdx::default(),
            children: TemplateBody::new(children),
            name,
            generics,
            fields,
            brace,
            spreads,
            diagnostics,
        };

        // We've received a valid rsx block, but it's not necessarily a valid component
        // validating it will dump diagnostics into the output
        component.validate_component_path();
        component.validate_fields();
        component.validate_key();
        component.validate_component_spread();

        Ok(component)
    }
}

impl ToTokens for Component {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let Self { name, generics, .. } = self;

        // Create props either from manual props or from the builder approach
        let props = self.create_props();

        // Make sure we stringify the component name
        let fn_name = self.fn_name().to_string();

        // Make sure we emit any errors
        let diagnostics = &self.diagnostics;

        tokens.append_all(quote! {
            dioxus_core::DynamicNode::Component({

                // todo: ensure going through the trait actually works
                // we want to avoid importing traits
                // use dioxus_core::prelude::Properties;
                use dioxus_core::prelude::Properties;
                let __comp = ({
                    #props
                }).into_vcomponent(
                    #name #generics,
                    #fn_name
                );
                #diagnostics
                __comp
            })
        })
    }
}

impl Component {
    // Make sure this a proper component path (uppercase ident, a path, or contains an underscorea)
    // This should be validated by the RsxBlock parser when it peeks bodynodes
    fn validate_component_path(&mut self) {
        let path = &self.name;

        // First, ensure the path is not a single lowercase ident with no underscores
        if path.segments.len() == 1 {
            let seg = path.segments.first().unwrap();
            if seg.ident.to_string().chars().next().unwrap().is_lowercase()
                && !seg.ident.to_string().contains('_')
            {
                self.diagnostics.push(seg.ident.span().error(
                    "Component names must be uppercase, contain an underscore, or abe a path.",
                ));
            }
        }

        // ensure path segments doesn't have PathArguments, only the last
        // segment is allowed to have one.
        if path
            .segments
            .iter()
            .take(path.segments.len() - 1)
            .any(|seg| seg.arguments != PathArguments::None)
        {
            self.diagnostics.push(path.span().error(
                "Component names must not have path arguments. Only the last segment is allowed to have one.",
            ));
        }

        // ensure last segment only have value of None or AngleBracketed
        if !matches!(
            path.segments.last().unwrap().arguments,
            PathArguments::None | PathArguments::AngleBracketed(_)
        ) {
            self.diagnostics.push(
                path.span()
                    .error("Component names must have no arguments or angle bracketed arguments."),
            );
        }
    }

    // Make sure the spread argument is being used as props spreading
    fn validate_component_spread(&mut self) {
        // Next, ensure that there's only one spread argument in the attributes *and* it's the last one
        for spread in self.spreads.iter().skip(1) {
            self.diagnostics.push(
                spread
                    .expr
                    .span()
                    .error("Only one set of manual props is allowed for a component."),
            );
        }
    }

    /// Ensure only one key and that the key is not a static str
    ///
    /// todo: we want to allow arbitrary exprs for keys provided they impl hash / eq
    fn validate_key(&mut self) {
        let key = self.get_key();

        if let Some(attr) = key {
            let diagnostic = match &attr.value {
                AttributeValue::AttrLiteral(ifmt) if ifmt.is_static() => {
                    ifmt.span().error("Key must not be a static string. Make sure to use a formatted string like `key: \"{value}\"")
                }
                AttributeValue::AttrLiteral(_) => return,
                _ => attr
                    .value
                    .span()
                    .error("Key must be in the form of a formatted string like `key: \"{value}\""),
            };

            self.diagnostics.push(diagnostic);
        }
    }

    pub fn get_key(&self) -> Option<&Attribute> {
        self.fields
            .iter()
            .find(|attr| matches!(&attr.name, AttributeName::BuiltIn(key) if key == "key"))
    }

    /// Ensure there's no duplicate props - this will be a compile error but we can move it to a
    /// diagnostic, thankfully
    ///
    /// Also ensure there's no stringly typed propsa
    fn validate_fields(&mut self) {
        let mut seen = HashSet::new();

        for field in self.fields.iter() {
            match &field.name {
                AttributeName::Custom(name) => self.diagnostics.push(
                    name.span()
                        .error("Custom attributes are not supported for Components. Only known attributes are allowed."),
                ),
                AttributeName::BuiltIn(k) => {
                    if !seen.contains(k) {
                        seen.insert(k);
                    } else {
                        self.diagnostics.push(
                            k.span()
                                .error("Duplicate prop field found. Only one prop field per name is allowed."),
                        );
                    }
                },
                AttributeName::Spread(_) => {
                    unreachable!("Spread attributes should be handled in the spread validation step.")
                }
            }
        }
    }

    /// Create the tokens we'll use for the props of the component
    ///
    /// todo: don't create the tokenstream from scratch and instead dump it into the existing streama
    fn create_props(&self) -> TokenStream2 {
        let manual_props = self.manual_props();

        let name = &self.name;
        let generics = &self.generics;

        let mut tokens = if let Some(props) = manual_props.as_ref() {
            quote! { let mut __manual_props = #props; }
        } else {
            quote! { fc_to_builder(#name #generics) }
        };

        for (name, value) in self.make_field_idents() {
            if manual_props.is_some() {
                tokens.append_all(quote! { __manual_props.#name = #value; })
            } else {
                tokens.append_all(quote! { .#name(#value) })
            }
        }

        if !self.children.is_empty() {
            let children = &self.children;
            if manual_props.is_some() {
                tokens.append_all(quote! { __manual_props.children = { #children }; })
            } else {
                tokens.append_all(quote! { .children( { #children } ) })
            }
        }

        if manual_props.is_some() {
            tokens.append_all(quote! { __manual_props })
        } else {
            tokens.append_all(quote! { .build() })
        }

        tokens
    }

    fn manual_props(&self) -> Option<&Expr> {
        self.spreads.first().map(|spread| &spread.expr)
    }

    fn make_field_idents(&self) -> Vec<(TokenStream2, TokenStream2)> {
        self.fields
            .iter()
            .filter_map(|attr| {
                let Attribute { name, value, .. } = attr;

                let attr = match name {
                    AttributeName::BuiltIn(k) => {
                        if k == "key" {
                            return None;
                        }
                        quote! { #k }
                    }
                    AttributeName::Custom(_) => return None,
                    AttributeName::Spread(_) => return None,
                };

                Some((attr, value.to_token_stream()))
            })
            .collect()
    }

    fn fn_name(&self) -> Ident {
        self.name.segments.last().unwrap().ident.clone()
    }

    fn empty(name: syn::Path, generics: Option<AngleBracketedGenericArguments>) -> Self {
        let mut diagnostics = Diagnostics::new();
        diagnostics.push(
            name.span()
                .error("Components must have a body")
                .help("Components must have a body, for example `Component {}`"),
        );
        Component {
            name,
            generics,
            brace: token::Brace::default(),
            fields: vec![],
            spreads: vec![],
            children: TemplateBody::new(vec![]),
            dyn_idx: DynIdx::default(),
            diagnostics,
        }
    }
}

/// Normalize the generics of a path
///
/// Ensure there's a `::` after the last segment if there are generics
fn normalize_path(name: &mut syn::Path) -> Option<AngleBracketedGenericArguments> {
    let seg = name.segments.last_mut()?;

    let mut generics = match seg.arguments.clone() {
        PathArguments::AngleBracketed(args) => {
            seg.arguments = PathArguments::None;
            Some(args)
        }
        _ => None,
    };

    if let Some(generics) = generics.as_mut() {
        use syn::Token;
        generics.colon2_token = Some(Token![::](proc_macro2::Span::call_site()));
    }

    generics
}

/// Ensure we can parse a component
#[test]
fn parses() {
    let input = quote! {
        MyComponent {
            key: "value {something}",
            prop: "value",
            ..props,
            div {
                "Hello, world!"
            }
        }
    };

    let component: Component = syn::parse2(input).unwrap();

    dbg!(component);

    let input_without_manual_props = quote! {
        MyComponent {
            key: "value {something}",
            prop: "value",
            div { "Hello, world!" }
        }
    };

    let component: Component = syn::parse2(input_without_manual_props).unwrap();
    dbg!(component);
}

/// Ensure we reject invalid forms
///
/// Maybe want to snapshot the errors?
#[test]
fn rejects() {
    let input = quote! {
        myComponent {
            key: "value",
            prop: "value",
            prop: "other",
            ..props,
            ..other_props,
            div {
                "Hello, world!"
            }
        }
    };

    let component: Component = syn::parse2(input).unwrap();
    dbg!(component.diagnostics);
}

#[test]
fn to_tokens_properly() {
    let input = quote! {
        MyComponent {
            key: "value {something}",
            prop: "value",
            prop: "value",
            prop: "value",
            prop: "value",
            prop: 123,
            ..props,
            div { "Hello, world!" }
        }
    };

    let component: Component = syn::parse2(input).unwrap();
    println!("{}", component.to_token_stream());
}

#[test]
fn to_tokens_no_manual_props() {
    let input_without_manual_props = quote! {
        MyComponent {
            key: "value {something}",
            named: "value {something}",
            prop: "value",
            count: 1,
            div { "Hello, world!" }
        }
    };
    let component: Component = syn::parse2(input_without_manual_props).unwrap();
    println!("{}", component.to_token_stream().pretty_unparse());
}

#[test]
fn generics_params() {
    let input_without_children = quote! {
         Outlet::<R> {}
    };
    let component: CallBody = syn::parse2(input_without_children).unwrap();
    println!("{}", component.to_token_stream().pretty_unparse());
}

#[test]
fn generics_no_fish() {
    let name = quote! { Outlet<R> };
    let mut p = syn::parse2::<syn::Path>(name).unwrap();
    let generics = normalize_path(&mut p);
    assert!(generics.is_some());

    let input_without_children = quote! {
        div {
            Component<Generic> {}
        }
    };
    let component: BodyNode = syn::parse2(input_without_children).unwrap();
    println!("{}", component.to_token_stream().pretty_unparse());
}

#[test]
fn fmt_passes_properly() {
    let input = quote! {
        Link { to: Route::List, class: "pure-button", "Go back" }
    };

    let component: Component = syn::parse2(input).unwrap();

    println!("{}", component.to_token_stream().pretty_unparse());
}

#[test]
fn incomplete_components() {
    let input = quote::quote! {
        some::cool::Component
    };

    let _parsed: Component = syn::parse2(input).unwrap();

    let input = quote::quote! {
        some::cool::C
    };

    let _parsed: syn::Path = syn::parse2(input).unwrap();
}
