extern crate proc_macro;

use layout::Layout;
use nest::{Nest, NestId};
use proc_macro::TokenStream;
use quote::{__private::Span, format_ident, quote, ToTokens};
use route::Route;
use syn::{parse::ParseStream, parse_macro_input, Ident};

use proc_macro2::TokenStream as TokenStream2;

use crate::{layout::LayoutId, route_tree::RouteTree};

mod layout;
mod nest;
mod query;
mod route;
mod route_tree;
mod segment;

// #[proc_macro_derive(Routable, attributes(route, nest, end_nest))]
#[proc_macro_attribute]
pub fn routable(_: TokenStream, input: TokenStream) -> TokenStream {
    let routes_enum = parse_macro_input!(input as syn::ItemEnum);

    let route_enum = match RouteEnum::parse(routes_enum) {
        Ok(route_enum) => route_enum,
        Err(err) => return err.to_compile_error().into(),
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
    attrs: Vec<syn::Attribute>,
    vis: syn::Visibility,
    name: Ident,
    routes: Vec<Route>,
    nests: Vec<Nest>,
    layouts: Vec<Layout>,
}

impl RouteEnum {
    fn parse(data: syn::ItemEnum) -> syn::Result<Self> {
        let name = &data.ident;

        let mut routes = Vec::new();

        let mut layouts = Vec::new();
        let mut layout_stack = Vec::new();

        let mut nests = Vec::new();
        let mut nest_stack = Vec::new();

        for variant in &data.variants {
            // Apply the any nesting attributes in order
            for attr in &variant.attrs {
                if attr.path.is_ident("nest") {
                    let mut children_routes = Vec::new();
                    {
                        // add all of the variants of the enum to the children_routes until we hit an end_nest
                        let mut level = 0;
                        'o: for variant in &data.variants {
                            children_routes.push(variant.fields.clone());
                            for attr in &variant.attrs {
                                if attr.path.is_ident("nest") {
                                    level += 1;
                                } else if attr.path.is_ident("end_nest") {
                                    level -= 1;
                                    if level < 0 {
                                        break 'o;
                                    }
                                }
                            }
                        }
                    }

                    let nest_index = nests.len();

                    let parser = |input: ParseStream| {
                        Nest::parse(
                            input,
                            children_routes
                                .iter()
                                .filter_map(|f: &syn::Fields| match f {
                                    syn::Fields::Named(fields) => Some(fields.clone()),
                                    _ => None,
                                })
                                .collect(),
                            nest_index,
                        )
                    };
                    let nest = attr.parse_args_with(parser)?;

                    nests.push(nest);
                    nest_stack.push(NestId(nest_index));
                } else if attr.path.is_ident("end_nest") {
                    nest_stack.pop();
                } else if attr.path.is_ident("layout") {
                    let layout_index = layouts.len();

                    let parser = |input: ParseStream| {
                        Layout::parse(input, nest_stack.iter().rev().cloned().collect())
                    };
                    let layout = attr.parse_args_with(parser)?;

                    layouts.push(layout);
                    layout_stack.push(LayoutId(layout_index));
                } else if attr.path.is_ident("end_layout") {
                    layout_stack.pop();
                }
            }

            let mut active_nests = nest_stack.clone();
            active_nests.reverse();
            let mut active_layouts = layout_stack.clone();
            active_layouts.reverse();

            let route = Route::parse(active_nests, active_layouts, variant.clone())?;

            routes.push(route);
        }

        let myself = Self {
            name: name.clone(),
            attrs: data.attrs,
            vis: data.vis,
            routes,
            nests,
            layouts,
        };

        Ok(myself)
    }

    fn impl_display(&self) -> TokenStream2 {
        let mut display_match = Vec::new();

        for route in &self.routes {
            display_match.push(route.display_match(&self.nests));
        }

        let name = &self.name;

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
        let tree = RouteTree::new(&self.routes, &self.nests);
        let name = &self.name;

        let error_name = format_ident!("{}MatchError", self.name);
        let tokens = tree.roots.iter().map(|&id| {
            let route = tree.get(id).unwrap();
            route.to_tokens(&tree, self.name.clone(), error_name.clone())
        });

        quote! {
            impl<'a> TryFrom<&'a str> for #name {
                type Error = <Self as std::str::FromStr>::Err;

                fn try_from(s: &'a str) -> Result<Self, Self::Error> {
                    s.parse()
                }
            }

            impl std::str::FromStr for #name {
                type Err = dioxus_router::routable::RouteParseError<#error_name>;

                fn from_str(s: &str) -> Result<Self, Self::Err> {
                    let route = s.strip_prefix('/').unwrap_or(s);
                    let (route, query) = route.split_once('?').unwrap_or((route, ""));
                    let mut segments = route.split('/');
                    let mut errors = Vec::new();

                    #(#tokens)*

                    Err(dioxus_router::routable::RouteParseError {
                        attempted_routes: errors,
                    })
                }
            }
        }
    }

    fn error_name(&self) -> Ident {
        Ident::new(&(self.name.to_string() + "MatchError"), Span::call_site())
    }

    fn error_type(&self) -> TokenStream2 {
        let match_error_name = self.error_name();

        let mut type_defs = Vec::new();
        let mut error_variants = Vec::new();
        let mut display_match = Vec::new();

        for route in &self.routes {
            let route_name = &route.route_name;

            let error_name = route.error_ident();
            let route_str = &route.route;

            error_variants.push(quote! { #route_name(#error_name) });
            display_match.push(quote! { Self::#route_name(err) => write!(f, "Route '{}' ('{}') did not match:\n{}", stringify!(#route_name), #route_str, err)? });
            type_defs.push(route.error_type());
        }

        for nest in &self.nests {
            let error_variant = nest.error_variant();
            let error_name = nest.error_ident();
            let route_str = &nest.route;

            error_variants.push(quote! { #error_variant(#error_name) });
            display_match.push(quote! { Self::#error_variant(err) => write!(f, "Nest '{}' ('{}') did not match:\n{}", stringify!(#error_name), #route_str, err)? });
            type_defs.push(nest.error_type());
        }

        quote! {
            #(#type_defs)*

            #[allow(non_camel_case_types)]
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
        let name = &self.name;

        let mut layers = Vec::new();

        loop {
            let index = layers.len();
            let mut routable_match = Vec::new();

            // Collect all routes that match the current layer
            for route in &self.routes {
                if let Some(matched) = route.routable_match(&self.layouts, &self.nests, index) {
                    routable_match.push(matched);
                }
            }

            // All routes are exhausted
            if routable_match.is_empty() {
                break;
            }

            layers.push(quote! {
                #(#routable_match)*
            });
        }

        let index_iter = 0..layers.len();

        quote! {
            impl dioxus_router::routable::Routable for #name where Self: Clone {
                fn render<'a>(&self, cx: &'a ScopeState, level: usize) -> Element<'a> {
                    let myself = self.clone();
                    match level {
                        #(
                            #index_iter => {
                                match myself {
                                    #layers
                                    _ => panic!("Route::render called with invalid level {}", level),
                                }
                            },
                        )*
                        _ => panic!("Route::render called with invalid level {}", level),
                    }
                }
            }
        }
    }
}

impl ToTokens for RouteEnum {
    fn to_tokens(&self, tokens: &mut quote::__private::TokenStream) {
        let routes = &self.routes;
        let vis = &self.vis;
        let name = &self.name;
        let attrs = &self.attrs;
        let variants = routes.iter().map(|r| r.variant());

        tokens.extend(quote!(
            #(#attrs)*
            #vis enum #name {
                #(#variants),*
            }

            #[path = "pages"]
            mod pages {
                #(#routes)*
            }
            pub use pages::*;
        ));
    }
}
