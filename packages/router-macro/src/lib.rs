#![doc = include_str!("../README.md")]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]

extern crate proc_macro;

use layout::Layout;
use nest::{Nest, NestId};
use proc_macro::TokenStream;
use quote::{__private::Span, format_ident, quote, ToTokens};
use redirect::Redirect;
use route::{Route, RouteType};
use segment::RouteSegment;
use syn::{parse::ParseStream, parse_macro_input, Ident, Token, Type};

use proc_macro2::TokenStream as TokenStream2;

use crate::{layout::LayoutId, route_tree::RouteTree};

mod layout;
mod nest;
mod query;
mod redirect;
mod route;
mod route_tree;
mod segment;

/// Derives the Routable trait for an enum of routes
///
/// Each variant must:
/// 1. Be struct-like with {}'s
/// 2. Contain all of the dynamic parameters of the current and nested routes
/// 3. Have a `#[route("route")]` attribute
///
/// Route Segments:
/// 1. Static Segments: "/static"
/// 2. Dynamic Segments: "/:dynamic" (where dynamic has a type that is FromStr in all child Variants)
/// 3. Catch all Segments: "/:..segments" (where segments has a type that is FromSegments in all child Variants)
/// 4. Query Segments: "/?:..query" (where query has a type that is FromQuery in all child Variants) or "/?:query&:other_query" (where query and other_query has a type that is FromQueryArgument in all child Variants)
///
/// Routes are matched:
/// 1. By there specificity this order: Query Routes ("/?:query"), Static Routes ("/route"), Dynamic Routes ("/:route"), Catch All Routes ("/:..route")
/// 2. By the order they are defined in the enum
///
/// All features:
/// ```rust, skip
/// #[rustfmt::skip]
/// #[derive(Clone, Debug, PartialEq, Routable)]
/// enum Route {
///     // Define routes with the route macro. If the name of the component is not the same as the variant, you can specify it as the second parameter
///     #[route("/", IndexComponent)]
///     Index {},
///     // Nests with parameters have types taken from child routes
///     // Everything inside the nest has the added parameter `user_id: usize`
///     #[nest("/user/:user_id")]
///         // All children of layouts will be rendered inside the Outlet in the layout component
///         // Creates a Layout UserFrame that has the parameter `user_id: usize`
///         #[layout(UserFrame)]
///             // If there is a component with the name Route1, you do not need to pass in the component name
///             #[route("/:dynamic?:query")]
///             Route1 {
///                 // The type is taken from the first instance of the dynamic parameter
///                 user_id: usize,
///                 dynamic: usize,
///                 query: String,
///                 extra: String,
///             },
///             #[route("/hello_world")]
///             // You can opt out of the layout by using the `!` prefix
///             #[layout(!UserFrame)]
///             Route2 { user_id: usize },
///         // End layouts with #[end_layout]
///         #[end_layout]
///     // End nests with #[end_nest]
///     #[end_nest]
///     // Redirects take a path and a function that takes the parameters from the path and returns a new route
///     #[redirect("/:id/user", |id: usize| Route::Route3 { dynamic: id.to_string()})]
///     #[route("/:dynamic")]
///     Route3 { dynamic: String },
///     #[child]
///     NestedRoute(NestedRoute),
/// }
/// ```
///
/// # `#[route("path", component)]`
///
/// The `#[route]` attribute is used to define a route. It takes up to 2 parameters:
/// - `path`: The path to the enum variant (relative to the parent nest)
/// - (optional) `component`: The component to render when the route is matched. If not specified, the name of the variant is used
///
/// Routes are the most basic attribute. They allow you to define a route and the component to render when the route is matched. The component must take all dynamic parameters of the route and all parent nests.
/// The next variant will be tied to the component. If you link to that variant, the component will be rendered.
///
/// ```rust, skip
/// #[derive(Clone, Debug, PartialEq, Routable)]
/// enum Route {
///     // Define routes that renders the IndexComponent
///     // The Index component will be rendered when the route is matched (e.g. when the user navigates to /)
///     #[route("/", Index)]
///     Index {},
/// }
/// ```
///
/// # `#[redirect("path", function)]`
///
/// The `#[redirect]` attribute is used to define a redirect. It takes 2 parameters:
/// - `path`: The path to the enum variant (relative to the parent nest)
/// - `function`: A function that takes the parameters from the path and returns a new route
///
/// ```rust, skip
/// #[derive(Clone, Debug, PartialEq, Routable)]
/// enum Route {
///     // Redirects the /:id route to the Index route
///     #[redirect("/:id", |_: usize| Route::Index {})]
///     #[route("/", Index)]
///     Index {},
/// }
/// ```
///
/// Redirects allow you to redirect a route to another route. The function must take all dynamic parameters of the route and all parent nests.
///
/// # `#[nest("path")]`
///
/// The `#[nest]` attribute is used to define a nest. It takes 1 parameter:
/// - `path`: The path to the nest (relative to the parent nest)
///
/// Nests effect all nests, routes and redirects defined until the next `#[end_nest]` attribute. All children of nests are relative to the nest route and must include all dynamic parameters of the nest.
///
/// ```rust, skip
/// #[derive(Clone, Debug, PartialEq, Routable)]
/// enum Route {
///     // Nests all child routes in the /blog route
///     #[nest("/blog")]
///         // This is at /blog/:id
///         #[redirect("/:id", |_: usize| Route::Index {})]
///         // This is at /blog
///         #[route("/", Index)]
///         Index {},
/// }
/// ```
///
/// # `#[end_nest]`
///
/// The `#[end_nest]` attribute is used to end a nest. It takes no parameters.
///
/// ```rust, skip
/// #[derive(Clone, Debug, PartialEq, Routable)]
/// enum Route {
///     #[nest("/blog")]
///         // This is at /blog/:id
///         #[redirect("/:id", |_: usize| Route::Index {})]
///         // This is at /blog
///         #[route("/", Index)]
///         Index {},
///     // Ends the nest
///     #[end_nest]
///     // This is at /
///     #[route("/")]
///     Home {},
/// }
/// ```
///
/// # `#[layout(component)]`
///
/// The `#[layout]` attribute is used to define a layout. It takes 1 parameter:
/// - `component`: The component to render when the route is matched. If not specified, the name of the variant is used
///
/// The layout component allows you to wrap all children of the layout in a component. The child routes are rendered in the Outlet of the layout component. The layout component must take all dynamic parameters of the nests it is nested in.
///
/// ```rust, skip
/// #[derive(Clone, Debug, PartialEq, Routable)]
/// enum Route {
///     #[layout(BlogFrame)]
///         #[redirect("/:id", |_: usize| Route::Index {})]
///         // Index will be rendered in the Outlet of the BlogFrame component
///         #[route("/", Index)]
///         Index {},
/// }
/// ```
///
/// # `#[end_layout]`
///
/// The `#[end_layout]` attribute is used to end a layout. It takes no parameters.
///
/// ```rust, skip
/// #[derive(Clone, Debug, PartialEq, Routable)]
/// enum Route {
///     #[layout(BlogFrame)]
///         #[redirect("/:id", |_: usize| Route::Index {})]
///         // Index will be rendered in the Outlet of the BlogFrame component
///         #[route("/", Index)]
///         Index {},
///     // Ends the layout
///     #[end_layout]
///     // This will be rendered standalone
///     #[route("/")]
///     Home {},
/// }
/// ```
#[proc_macro_derive(
    Routable,
    attributes(route, nest, end_nest, layout, end_layout, redirect, child)
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
    let component_impl = route_enum.component_impl();

    (quote! {
        #error_type

        #display_impl

        #routable_impl

        #component_impl

        #parse_impl
    })
    .into()
}

struct RouteEnum {
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
                if attr.path().is_ident("nest") {
                    let mut children_routes = Vec::new();
                    {
                        // add all of the variants of the enum to the children_routes until we hit an end_nest
                        let mut level = 0;
                        'o: for variant in &data.variants {
                            children_routes.push(variant.fields.clone());
                            for attr in &variant.attrs {
                                if attr.path().is_ident("nest") {
                                    level += 1;
                                } else if attr.path().is_ident("end_nest") {
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
                } else if attr.path().is_ident("end_nest") {
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
                } else if attr.path().is_ident("layout") {
                    let parser = |input: ParseStream| {
                        let bang: Option<Token![!]> = input.parse().ok();
                        let exclude = bang.is_some();
                        Ok((exclude, Layout::parse(input, nest_stack.clone())?))
                    };
                    let (exclude, layout): (bool, Layout) = attr.parse_args_with(parser)?;

                    if exclude {
                        let Some(layout_index) = layouts.iter().position(|l| l.comp == layout.comp)
                        else {
                            return Err(syn::Error::new(
                                Span::call_site(),
                                "Attempted to exclude a layout that does not exist",
                            ));
                        };
                        excluded.push(LayoutId(layout_index));
                    } else {
                        let layout_index = layouts.len();
                        layouts.push(layout);
                        layout_stack.push(LayoutId(layout_index));
                    }
                } else if attr.path().is_ident("end_layout") {
                    layout_stack.pop();
                } else if attr.path().is_ident("redirect") {
                    let parser = |input: ParseStream| {
                        Redirect::parse(input, nest_stack.clone(), redirects.len())
                    };
                    let redirect = attr.parse_args_with(parser)?;
                    redirects.push(redirect);
                }
            }

            let active_nests = nest_stack.clone();
            let mut active_layouts = layout_stack.clone();
            active_layouts.retain(|&id| !excluded.contains(&id));

            let route = Route::parse(active_nests, active_layouts, variant.clone())?;

            // add the route to the site map
            let mut segment = SiteMapSegment::new(&route.segments);
            if let RouteType::Child(child) = &route.ty {
                let new_segment = SiteMapSegment {
                    segment_type: SegmentType::Child(child.ty.clone()),
                    children: Vec::new(),
                };
                match &mut segment {
                    Some(segment) => {
                        fn set_last_child_to(
                            segment: &mut SiteMapSegment,
                            new_segment: SiteMapSegment,
                        ) {
                            if let Some(last) = segment.children.last_mut() {
                                set_last_child_to(last, new_segment);
                            } else {
                                segment.children = vec![new_segment];
                            }
                        }
                        set_last_child_to(segment, new_segment);
                    }
                    None => {
                        segment = Some(new_segment);
                    }
                }
            }

            if let Some(segment) = segment {
                let parent = site_map_stack.last_mut();
                let children = match parent {
                    Some(parent) => &mut parent.last_mut().unwrap().children,
                    None => &mut site_map,
                };
                children.push(segment);
            }

            routes.push(route);
        }

        // pop any remaining site map segments
        while let Some(segment) = site_map_stack.pop() {
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

        let myself = Self {
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
                    let (route, _hash) = route.split_once('#').unwrap_or((route, ""));
                    let (route, query) = route.split_once('?').unwrap_or((route, ""));
                    let query = dioxus_router::exports::urlencoding::decode(query).unwrap_or(query.into());
                    let mut segments = route.split('/').map(|s| dioxus_router::exports::urlencoding::decode(s).unwrap_or(s.into()));
                    // skip the first empty segment
                    if s.starts_with('/') {
                        let _ = segments.next();
                    }
                    else {
                        // if this route does not start with a slash, it is not a valid route
                        return Err(dioxus_router::routable::RouteParseError {
                            attempted_routes: Vec::new(),
                        });
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

        let mut matches = Vec::new();

        // Collect all routes matches
        for route in &self.routes {
            matches.push(route.routable_match(&self.layouts, &self.nests));
        }

        quote! {
            impl dioxus_router::routable::Routable for #name where Self: Clone {
                const SITE_MAP: &'static [dioxus_router::routable::SiteMapSegment] = &[
                    #(#site_map,)*
                ];

                fn render(&self, level: usize) -> ::dioxus::prelude::Element {
                    let myself = self.clone();
                    match (level, myself) {
                        #(#matches)*
                        _ => None
                    }
                }
            }
        }
    }

    fn component_impl(&self) -> TokenStream2 {
        let name = &self.name;
        let props = quote! { ::std::rc::Rc<::std::cell::Cell<dioxus_router::prelude::RouterConfig<#name>>> };

        quote! {
            impl dioxus_core::ComponentFunction<#props> for #name {
                fn rebuild(&self, props: #props) -> dioxus_core::Element {
                    let initial_route = self.clone();
                    rsx! {
                        dioxus_router::prelude::Router::<#name> {
                            config: move || props.take().initial_route(initial_route)
                        }
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
        let children = if let SegmentType::Child(ty) = &self.segment_type {
            quote! { #ty::SITE_MAP }
        } else {
            let children = self
                .children
                .iter()
                .map(|child| child.to_token_stream())
                .collect::<Vec<_>>();
            quote! {
                &[
                    #(#children,)*
                ]
            }
        };

        tokens.extend(quote! {
            dioxus_router::routable::SiteMapSegment {
                segment_type: #segment_type,
                children: #children,
            }
        });
    }
}

enum SegmentType {
    Static(String),
    Dynamic(String),
    CatchAll(String),
    Child(Type),
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
            SegmentType::Child(_) => {
                tokens.extend(quote! { dioxus_router::routable::SegmentType::Child })
            }
        }
    }
}

impl<'a> From<&'a RouteSegment> for SegmentType {
    fn from(value: &'a RouteSegment) -> Self {
        match value {
            RouteSegment::Static(s) => SegmentType::Static(s.to_string()),
            RouteSegment::Dynamic(s, _) => SegmentType::Dynamic(s.to_string()),
            RouteSegment::CatchAll(s, _) => SegmentType::CatchAll(s.to_string()),
        }
    }
}
