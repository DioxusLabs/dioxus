extern crate proc_macro;

use layout::Layout;
use nest::{Nest, NestId};
use proc_macro::TokenStream;
use quote::{__private::Span, format_ident, quote, ToTokens};
use redirect::Redirect;
use route::Route;
use segment::RouteSegment;
use syn::{parse::ParseStream, parse_macro_input, Ident, Token};

use proc_macro2::TokenStream as TokenStream2;

use crate::{layout::LayoutId, route_tree::RouteTree};

mod layout;
mod nest;
mod query;
mod redirect;
mod route;
mod route_tree;
mod segment;

#[proc_macro_derive(
    Routable,
    attributes(route, nest, end_nest, layout, end_layout, redirect)
)]
pub fn routable(input: TokenStream) -> TokenStream {
    let routes_enum = parse_macro_input!(input as syn::ItemEnum);

    let route_enum = match RouteEnum::parse(routes_enum) {
        Ok(route_enum) => route_enum,
        Err(err) => return err.to_compile_error().into(),
    };

    let error_type = route_enum.error_type();
    let parse_impl = route_enum.parse_impl();
    let display_impl = route_enum.impl_display();
    let routable_impl = route_enum.routable_impl();
    let name = &route_enum.name;
    let vis = &route_enum.vis;

    quote! {
        #vis fn Outlet(cx: dioxus::prelude::Scope) -> dioxus::prelude::Element {
            dioxus_router::prelude::GenericOutlet::<#name>(cx)
        }

        #vis fn Router(cx: dioxus::prelude::Scope<dioxus_router::prelude::GenericRouterProps<#name>>) -> dioxus::prelude::Element {
            dioxus_router::prelude::GenericRouter(cx)
        }

        #vis fn Link<'a>(cx: dioxus::prelude::Scope<'a, dioxus_router::prelude::GenericLinkProps<'a, #name>>) -> dioxus::prelude::Element<'a> {
            dioxus_router::prelude::GenericLink(cx)
        }

        #vis fn use_router<R: dioxus_router::prelude::Routable + Clone>(cx: &dioxus::prelude::ScopeState) -> &dioxus_router::prelude::GenericRouterContext<R> {
            dioxus_router::prelude::use_generic_router::<R>(cx)
        }

        #error_type

        #parse_impl

        #display_impl

        #routable_impl
    }
    .into()
}

struct RouteEnum {
    vis: syn::Visibility,
    name: Ident,
    redirects: Vec<Redirect>,
    routes: Vec<Route>,
    nests: Vec<Nest>,
    layouts: Vec<Layout>,
    site_map: Vec<SiteMapSegment>,
}

impl RouteEnum {
    fn parse(data: syn::ItemEnum) -> syn::Result<Self> {
        let name = &data.ident;
        let vis = &data.vis;

        let mut site_map = Vec::new();
        let mut site_map_stack: Vec<Vec<SiteMapSegment>> = Vec::new();

        let mut routes = Vec::new();

        let mut redirects = Vec::new();

        let mut layouts: Vec<Layout> = Vec::new();
        let mut layout_stack = Vec::new();

        let mut nests = Vec::new();
        let mut nest_stack = Vec::new();

        for variant in &data.variants {
            let mut excluded = Vec::new();
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

                    // add the current segment to the site map stack
                    let segments: Vec<_> = nest
                        .segments
                        .iter()
                        .map(|seg| {
                            let segment_type = seg.into();
                            SiteMapSegment {
                                segment_type,
                                children: Vec::new(),
                            }
                        })
                        .collect();
                    if !segments.is_empty() {
                        site_map_stack.push(segments);
                    }

                    nests.push(nest);
                    nest_stack.push(NestId(nest_index));
                } else if attr.path.is_ident("end_nest") {
                    nest_stack.pop();
                    // pop the current nest segment off the stack and add it to the parent or the site map
                    if let Some(segment) = site_map_stack.pop() {
                        let children = site_map_stack
                            .last_mut()
                            .map(|seg| &mut seg.last_mut().unwrap().children)
                            .unwrap_or(&mut site_map);

                        // Turn the list of segments in the segments stack into a tree
                        let mut iter = segment.into_iter().rev();
                        let mut current = iter.next().unwrap();
                        for mut segment in iter {
                            segment.children.push(current);
                            current = segment;
                        }

                        children.push(current);
                    }
                } else if attr.path.is_ident("layout") {
                    let parser = |input: ParseStream| {
                        let bang: Option<Token![!]> = input.parse().ok();
                        let exclude = bang.is_some();
                        Ok((
                            exclude,
                            Layout::parse(input, nest_stack.iter().rev().cloned().collect())?,
                        ))
                    };
                    let (exclude, layout): (bool, Layout) = attr.parse_args_with(parser)?;

                    if exclude {
                        let Some(layout_index) =
                            layouts.iter().position(|l| l.comp == layout.comp)else{
                                return Err(syn::Error::new(
                                    Span::call_site(),
                                    "Attempted to exclude a layout that does not exist",
                                ));
                            }
                                ;
                        excluded.push(LayoutId(layout_index));
                    } else {
                        let layout_index = layouts.len();
                        layouts.push(layout);
                        layout_stack.push(LayoutId(layout_index));
                    }
                } else if attr.path.is_ident("end_layout") {
                    layout_stack.pop();
                } else if attr.path.is_ident("redirect") {
                    let parser = |input: ParseStream| {
                        Redirect::parse(
                            input,
                            nest_stack.iter().rev().cloned().collect(),
                            redirects.len(),
                        )
                    };
                    let redirect = attr.parse_args_with(parser)?;
                    redirects.push(redirect);
                }
            }

            let mut active_nests = nest_stack.clone();
            active_nests.reverse();
            let mut active_layouts = layout_stack.clone();
            active_layouts.retain(|&id| !excluded.contains(&id));
            active_layouts.reverse();

            let route = Route::parse(active_nests, active_layouts, variant.clone())?;

            // add the route to the site map
            if let Some(segment) = SiteMapSegment::new(&route.segments) {
                let parent = site_map_stack.last_mut();
                let children = match parent {
                    Some(parent) => &mut parent.last_mut().unwrap().children,
                    None => &mut site_map,
                };
                children.push(segment);
            }

            routes.push(route);
        }

        let myself = Self {
            vis: vis.clone(),
            name: name.clone(),
            routes,
            redirects,
            nests,
            layouts,
            site_map,
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
                    #[allow(unused)]
                    match self {
                        #(#display_match)*
                    }
                    Ok(())
                }
            }
        }
    }

    fn parse_impl(&self) -> TokenStream2 {
        let tree = RouteTree::new(&self.routes, &self.nests, &self.redirects);
        let name = &self.name;

        let error_name = format_ident!("{}MatchError", self.name);
        let tokens = tree.roots.iter().map(|&id| {
            let route = tree.get(id).unwrap();
            route.to_tokens(&self.nests, &tree, self.name.clone(), error_name.clone())
        });

        quote! {
            impl<'a> core::convert::TryFrom<&'a str> for #name {
                type Error = <Self as std::str::FromStr>::Err;

                fn try_from(s: &'a str) -> Result<Self, Self::Error> {
                    s.parse()
                }
            }

            impl std::str::FromStr for #name {
                type Err = dioxus_router::routable::RouteParseError<#error_name>;

                fn from_str(s: &str) -> Result<Self, Self::Err> {
                    let route = s;
                    let (route, query) = route.split_once('?').unwrap_or((route, ""));
                    let mut segments = route.split('/');
                    // skip the first empty segment
                    if s.starts_with('/') {
                        segments.next();
                    }
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

        for redirect in &self.redirects {
            let error_variant = redirect.error_variant();
            let error_name = redirect.error_ident();
            let route_str = &redirect.route;

            error_variants.push(quote! { #error_variant(#error_name) });
            display_match.push(quote! { Self::#error_variant(err) => write!(f, "Redirect '{}' ('{}') did not match:\n{}", stringify!(#error_name), #route_str, err)? });
            type_defs.push(redirect.error_type());
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
        let site_map = &self.site_map;

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
                const SITE_MAP: &'static [dioxus_router::routable::SiteMapSegment] = &[
                    #(#site_map,)*
                ];

                fn render<'a>(&self, cx: &'a ScopeState, level: usize) -> Element<'a> {
                    let myself = self.clone();
                    match level {
                        #(
                            #index_iter => {
                                match myself {
                                    #layers
                                    _ => None
                                }
                            },
                        )*
                        _ => None
                    }
                }
            }
        }
    }
}

struct SiteMapSegment {
    pub segment_type: SegmentType,
    pub children: Vec<SiteMapSegment>,
}

impl SiteMapSegment {
    fn new(segments: &[RouteSegment]) -> Option<Self> {
        let mut current = None;
        // walk backwards through the new segments, adding children as we go
        for segment in segments.iter().rev() {
            let segment_type = segment.into();
            let mut segment = SiteMapSegment {
                segment_type,
                children: Vec::new(),
            };
            // if we have a current segment, add it as a child
            if let Some(current) = current.take() {
                segment.children.push(current)
            }
            current = Some(segment);
        }
        current
    }
}

impl ToTokens for SiteMapSegment {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let segment_type = &self.segment_type;
        let children = &self.children;

        tokens.extend(quote! {
            dioxus_router::routable::SiteMapSegment {
                segment_type: #segment_type,
                children: &[
                    #(#children,)*
                ]
            }
        });
    }
}

enum SegmentType {
    Static(String),
    Dynamic(String),
    CatchAll(String),
}

impl ToTokens for SegmentType {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        match self {
            SegmentType::Static(s) => {
                tokens.extend(quote! { dioxus_router::routable::SegmentType::Static(#s) })
            }
            SegmentType::Dynamic(s) => {
                tokens.extend(quote! { dioxus_router::routable::SegmentType::Dynamic(#s) })
            }
            SegmentType::CatchAll(s) => {
                tokens.extend(quote! { dioxus_router::routable::SegmentType::CatchAll(#s) })
            }
        }
    }
}

impl<'a> From<&'a RouteSegment> for SegmentType {
    fn from(value: &'a RouteSegment) -> Self {
        match value {
            segment::RouteSegment::Static(s) => SegmentType::Static(s.to_string()),
            segment::RouteSegment::Dynamic(s, _) => SegmentType::Dynamic(s.to_string()),
            segment::RouteSegment::CatchAll(s, _) => SegmentType::CatchAll(s.to_string()),
        }
    }
}
