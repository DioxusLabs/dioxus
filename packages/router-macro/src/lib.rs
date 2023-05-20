extern crate proc_macro;

use nest::{Layout, Nest};
use proc_macro::TokenStream;
use quote::{__private::Span, format_ident, quote, ToTokens};
use route::Route;
use syn::{parse_macro_input, Ident};

use proc_macro2::TokenStream as TokenStream2;

use crate::{nest::LayoutId, route_tree::RouteTree};

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
    vis: syn::Visibility,
    attrs: Vec<syn::Attribute>,
    name: Ident,
    routes: Vec<Route>,
    layouts: Vec<Layout>,
}

impl RouteEnum {
    fn parse(data: syn::ItemEnum) -> syn::Result<Self> {
        let name = &data.ident;

        enum NestRef {
            Static(String),
            Dynamic { id: LayoutId },
        }

        let mut routes = Vec::new();

        let mut layouts = Vec::new();

        let mut nest_stack = Vec::new();

        for variant in data.variants {
            // Apply the any nesting attributes in order
            for attr in &variant.attrs {
                if attr.path.is_ident("nest") {
                    let nest: Nest = attr.parse_args()?;
                    let nest_ref = match nest {
                        Nest::Static(s) => NestRef::Static(s),
                        Nest::Layout(mut l) => {
                            // if there is a static nest before this, add it to the layout
                            let mut static_prefix = nest_stack
                                .iter()
                                // walk backwards and take all static nests
                                .rev()
                                .map_while(|nest| match nest {
                                    NestRef::Static(s) => Some(s.clone()),
                                    NestRef::Dynamic { .. } => None,
                                })
                                .collect::<Vec<_>>();
                            // reverse the static prefix so it is in the correct order
                            static_prefix.reverse();

                            if !static_prefix.is_empty() {
                                l.add_static_prefix(&static_prefix.join("/"));
                            }

                            let id = layouts.len();
                            layouts.push(l);
                            NestRef::Dynamic { id: LayoutId(id) }
                        }
                    };
                    nest_stack.push(nest_ref);
                } else if attr.path.is_ident("end_nest") {
                    nest_stack.pop();
                }
            }

            let mut trailing_static_route = nest_stack
                .iter()
                .rev()
                .map_while(|nest| match nest {
                    NestRef::Static(s) => Some(s.clone()),
                    NestRef::Dynamic { .. } => None,
                })
                .collect::<Vec<_>>();
            trailing_static_route.reverse();
            let active_layouts = nest_stack
                .iter()
                .filter_map(|nest| match nest {
                    NestRef::Static(_) => None,
                    NestRef::Dynamic { id } => Some(*id),
                })
                .collect::<Vec<_>>();

            let route = Route::parse(trailing_static_route.join("/"), active_layouts, variant)?;
            routes.push(route);
        }

        let myself = Self {
            vis: data.vis,
            attrs: data.attrs,
            name: name.clone(),
            routes,
            layouts,
        };

        Ok(myself)
    }

    fn impl_display(&self) -> TokenStream2 {
        let mut display_match = Vec::new();

        for route in &self.routes {
            display_match.push(route.display_match(&self.layouts));
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
        let tree = RouteTree::new(&self.routes, &self.layouts);
        let name = &self.name;

        let error_name = format_ident!("{}MatchError", self.name);
        let tokens = tree.roots.iter().map(|&id| {
            let route = tree.get(id).unwrap();
            route.to_tokens(&tree, self.name.clone(), error_name.clone(), &self.layouts)
        });

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

                    #(#tokens)*

                    Err(RouteParseError {
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

        for layout in &self.layouts {
            let layout_name = &layout.layout_name;

            let error_name = layout.error_ident();
            let route_str = &layout.route;

            error_variants.push(quote! { #layout_name(#error_name) });
            display_match.push(quote! { Self::#layout_name(err) => write!(f, "Layout '{}' ('{}') did not match:\n{}", stringify!(#layout_name), #route_str, err)? });
            type_defs.push(layout.error_type());
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
        let name = &self.name;

        let mut layers = Vec::new();

        loop {
            let index = layers.len();
            let mut routable_match = Vec::new();

            // Collect all routes that match the current layer
            for route in &self.routes {
                if let Some(matched) = route.routable_match(&self.layouts, index) {
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
            impl Routable for #name where Self: Clone {
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
        let variants = routes.iter().map(|r| r.variant(&self.layouts));

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
