extern crate proc_macro;

use proc_macro::TokenStream;
use quote::{__private::Span, format_ident, quote, ToTokens};
use route::Route;
use route_tree::RouteTreeSegment;
use syn::{parse_macro_input, Ident};

use proc_macro2::TokenStream as TokenStream2;

mod route;
mod route_tree;

#[proc_macro_derive(Routable, attributes(route))]
pub fn derive_routable(input: TokenStream) -> TokenStream {
    let routes_enum = parse_macro_input!(input as syn::DeriveInput);

    let route_enum = match RouteEnum::parse(routes_enum) {
        Ok(route_enum) => route_enum,
        Err(err) => return TokenStream2::from(err.to_compile_error()).into(),
    };

    let error_type = route_enum.error_type();
    let parse_impl = route_enum.parse_impl();
    let display_impl = route_enum.impl_display();
    let routable_impl = route_enum.routable_impl();

    quote! {
        #route_enum

        #error_type

        #parse_impl

        #display_impl

        #routable_impl
    }
    .into()
}

struct RouteEnum {
    route_name: Ident,
    routes: Vec<Route>,
}

impl RouteEnum {
    fn parse(input: syn::DeriveInput) -> syn::Result<Self> {
        let name = &input.ident;

        if let syn::Data::Enum(data) = input.data {
            let mut routes = Vec::new();

            for variant in data.variants {
                let route = Route::parse(variant)?;
                routes.push(route);
            }

            let myself = Self {
                route_name: name.clone(),
                routes,
            };

            Ok(myself)
        } else {
            Err(syn::Error::new_spanned(
                input.clone(),
                "Routable can only be derived for enums",
            ))
        }
    }

    fn impl_display(&self) -> TokenStream2 {
        let mut display_match = Vec::new();

        for route in &self.routes {
            display_match.push(route.display_match());
        }

        let name = &self.route_name;

        quote! {
            impl std::fmt::Display for #name {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    match self {
                        #(#display_match)*
                    }
                    Ok(())
                }
            }
        }
    }

    fn parse_impl(&self) -> TokenStream2 {
        let tree = RouteTreeSegment::build(&self.routes);
        let name = &self.route_name;

        let error_name = format_ident!("{}MatchError", self.route_name);
        let tokens = tree
            .into_iter()
            .map(|t| t.to_tokens(self.route_name.clone(), error_name.clone()));

        quote! {
            impl<'a> TryFrom<&'a str> for #name {
                type Error = <Self as std::str::FromStr>::Err;

                fn try_from(s: &'a str) -> Result<Self, Self::Error> {
                    s.parse()
                }
            }

            impl std::str::FromStr for #name {
                type Err = RouteParseError<#error_name>;

                fn from_str(s: &str) -> Result<Self, Self::Err> {
                    let route = s.strip_prefix('/').unwrap_or(s);
                    let (route, query) = route.split_once('?').unwrap_or((route, ""));
                    let mut segments = route.split('/');
                    let mut errors = Vec::new();

                    if let Some(segment) = segments.next() {
                        #(#tokens)*
                    }

                    Err(RouteParseError {
                        attempted_routes: errors,
                    })
                }
            }
        }
    }

    fn error_name(&self) -> Ident {
        Ident::new(
            &(self.route_name.to_string() + "MatchError"),
            Span::call_site(),
        )
    }

    fn error_type(&self) -> TokenStream2 {
        let match_error_name = self.error_name();

        let mut type_defs = Vec::new();
        let mut error_variants = Vec::new();
        let mut display_match = Vec::new();

        for route in &self.routes {
            let route_name = &route.route_name;

            let error_name = Ident::new(&format!("{}ParseError", route_name), Span::call_site());
            let route_str = &route.route;

            error_variants.push(quote! { #route_name(#error_name) });
            display_match.push(quote! { Self::#route_name(err) => write!(f, "Route '{}' ('{}') did not match:\n{}", stringify!(#route_name), #route_str, err)? });
            type_defs.push(route.error_type());
        }

        quote! {
            #(#type_defs)*

            #[derive(Debug, PartialEq)]
            pub enum #match_error_name {
                #(#error_variants),*
            }

            impl std::fmt::Display for #match_error_name {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    match self {
                        #(#display_match),*
                    }
                    Ok(())
                }
            }
        }
    }

    fn routable_impl(&self) -> TokenStream2 {
        let mut routable_match = Vec::new();

        for route in &self.routes {
            routable_match.push(route.routable_match());
        }

        quote! {
            impl Routable for Route {
                fn render<'a>(self, cx: &'a ScopeState) -> Element<'a> {
                    match self {
                        #(#routable_match)*
                    }
                }
            }
        }
    }
}

impl ToTokens for RouteEnum {
    fn to_tokens(&self, tokens: &mut quote::__private::TokenStream) {
        let routes = &self.routes;

        tokens.extend(quote!(
            #[path = "pages"]
            mod pages {
                #(#routes)*
            }
            pub use pages::*;
        ));
    }
}
