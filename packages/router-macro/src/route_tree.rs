use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

use crate::route::{static_segment_idx, Route, RouteSegment};

// First deduplicate the routes by the static part of the route
#[derive(Debug)]
pub enum RouteTreeSegment<'a> {
    Static {
        index: usize,
        segment: &'a str,
        children: Vec<RouteTreeSegment<'a>>,
        from_route: &'a Route,
    },
    Dynamic(&'a Route),
    StaticEnd(&'a Route),
}

impl<'a> RouteTreeSegment<'a> {
    pub fn build(routes: &'a Vec<Route>) -> Vec<RouteTreeSegment<'a>> {
        let routes = routes.into_iter().map(PartialRoute::new).collect();
        Self::construct(routes)
    }

    fn construct(routes: Vec<PartialRoute<'a>>) -> Vec<RouteTreeSegment<'a>> {
        let mut static_segments = Vec::new();
        let mut dyn_segments = Vec::new();

        // Add all routes we can to the tree
        for mut route in routes {
            match route.next_static_segment() {
                // If there is a static segment, check if it already exists in the tree
                Some((i, segment)) => {
                    let found = static_segments.iter_mut().find_map(|seg| match seg {
                        RouteTreeSegment::Static {
                            segment: s,
                            children,
                            ..
                        } => (s == &segment).then(|| children),
                        _ => None,
                    });

                    match found {
                        Some(children) => {
                            // If it does, add the route to the children of the segment
                            children.append(&mut RouteTreeSegment::construct(vec![route]))
                        }
                        None => {
                            // If it doesn't, add the route as a new segment
                            static_segments.push(RouteTreeSegment::Static {
                                segment,
                                from_route: route.route,
                                children: RouteTreeSegment::construct(vec![route]),
                                index: i,
                            })
                        }
                    }
                }
                // If there is no static segment, add the route to the dynamic routes
                None => {
                    // This route is entirely static
                    if route.route.route_segments.len() == route.static_segment_index {
                        static_segments.push(RouteTreeSegment::StaticEnd(route.route));
                    } else {
                        dyn_segments.push(RouteTreeSegment::Dynamic(route.route));
                    }
                }
            }
        }

        // All static routes are checked before dynamic routes
        static_segments.append(&mut dyn_segments);

        static_segments
    }
}

impl<'a> RouteTreeSegment<'a> {
    pub fn to_tokens(&self, enum_name: syn::Ident, error_enum_name: syn::Ident) -> TokenStream {
        match self {
            RouteTreeSegment::Static {
                segment,
                children,
                index,
                from_route,
            } => {
                let varient_parse_error = from_route.error_ident();
                let enum_varient = &from_route.route_name;
                let error_ident = static_segment_idx(*index);

                let children_with_next_segment = children.iter().filter_map(|child| match child {
                    RouteTreeSegment::StaticEnd { .. } => None,
                    _ => Some(child.to_tokens(enum_name.clone(), error_enum_name.clone())),
                });
                let children_without_next_segment =
                    children.iter().filter_map(|child| match child {
                        RouteTreeSegment::StaticEnd { .. } => {
                            Some(child.to_tokens(enum_name.clone(), error_enum_name.clone()))
                        }
                        _ => None,
                    });

                quote! {
                    if #segment == segment {
                        let mut segments = segments.clone();
                        #(#children_without_next_segment)*
                        if let Some(segment) = segments.next() {
                            #(#children_with_next_segment)*
                        }
                    }
                    else {
                        errors.push(#error_enum_name::#enum_varient(#varient_parse_error::#error_ident))
                    }
                }
            }
            RouteTreeSegment::Dynamic(route) => {
                // At this point, we have matched all static segments, so we can just check if the remaining segments match the route
                let varient_parse_error = route.error_ident();
                let enum_varient = &route.route_name;

                let route_segments = route
                    .route_segments
                    .iter()
                    .enumerate()
                    .skip_while(|(_, seg)| match seg {
                        RouteSegment::Static(_) => true,
                        _ => false,
                    })
                    .map(|(i, seg)| {
                        (
                            seg.name(),
                            seg.try_parse(i, &error_enum_name, enum_varient, &varient_parse_error),
                        )
                    });

                fn print_route_segment<I: Iterator<Item = (Option<Ident>, TokenStream)>>(
                    mut s: std::iter::Peekable<I>,
                    sucess_tokens: TokenStream,
                ) -> TokenStream {
                    if let Some((name, first)) = s.next() {
                        let has_next = s.peek().is_some();
                        let children = print_route_segment(s, sucess_tokens);
                        let name = name
                            .map(|name| quote! {#name})
                            .unwrap_or_else(|| quote! {_});

                        let sucess = if has_next {
                            quote! {
                                let mut segments = segments.clone();
                                if let Some(segment) = segments.next() {
                                    #children
                                }
                            }
                        } else {
                            children
                        };

                        quote! {
                            #first
                            match parsed {
                                Ok(#name) => {
                                    #sucess
                                }
                                Err(err) => {
                                    errors.push(err);
                                }
                            }
                        }
                    } else {
                        quote! {
                            #sucess_tokens
                        }
                    }
                }

                let construct_variant = route.construct(enum_name);

                print_route_segment(
                    route_segments.peekable(),
                    return_constructed(
                        construct_variant,
                        &error_enum_name,
                        enum_varient,
                        &varient_parse_error,
                    ),
                )
            }
            Self::StaticEnd(route) => {
                let varient_parse_error = route.error_ident();
                let enum_varient = &route.route_name;
                let construct_variant = route.construct(enum_name);

                return_constructed(
                    construct_variant,
                    &error_enum_name,
                    enum_varient,
                    &varient_parse_error,
                )
            }
        }
    }
}

fn return_constructed(
    construct_variant: TokenStream,
    error_enum_name: &Ident,
    enum_varient: &Ident,
    varient_parse_error: &Ident,
) -> TokenStream {
    quote! {
        let remaining_segments = segments.clone();
        let mut segments_clone = segments.clone();
        let next_segment = segments_clone.next();
        let segment_after_next = segments_clone.next();
        match (next_segment, segment_after_next) {
            // This is the last segment, return the parsed route
            (None, _) | (Some(""), None) => {
                return Ok(#construct_variant);
            }
            _ => {
                let mut trailing = String::new();
                for seg in remaining_segments {
                    trailing += seg;
                    trailing += "/";
                }
                trailing.pop();
                errors.push(#error_enum_name::#enum_varient(#varient_parse_error::ExtraSegments(trailing)))
            }
        }
    }
}

struct PartialRoute<'a> {
    route: &'a Route,
    static_segment_index: usize,
}

impl<'a> PartialRoute<'a> {
    fn new(route: &'a Route) -> Self {
        Self {
            route,
            static_segment_index: 0,
        }
    }

    fn next_static_segment(&mut self) -> Option<(usize, &'a str)> {
        let idx = self.static_segment_index;
        let segment = self.route.route_segments.get(idx)?;
        match segment {
            RouteSegment::Static(segment) => {
                self.static_segment_index += 1;
                Some((idx, segment))
            }
            _ => None,
        }
    }
}
