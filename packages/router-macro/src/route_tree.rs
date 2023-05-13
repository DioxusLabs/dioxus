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
    pub fn build(routes: &'a [Route]) -> Vec<RouteTreeSegment<'a>> {
        let routes = routes.iter().map(PartialRoute::new).collect();
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
                        } => (s == &segment).then_some(children),
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

                let children = children
                    .iter()
                    .map(|child| child.to_tokens(enum_name.clone(), error_enum_name.clone()));

                quote! {
                    {
                        let mut segments = segments.clone();
                        if let Some(segment) = segments.next() {
                            if #segment == segment {
                                #(#children)*
                            }
                            else {
                                errors.push(#error_enum_name::#enum_varient(#varient_parse_error::#error_ident))
                            }
                        }
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
                    .skip_while(|(_, seg)| matches!(seg, RouteSegment::Static(_)));

                fn print_route_segment<'a, I: Iterator<Item = (usize, &'a RouteSegment)>>(
                    mut s: std::iter::Peekable<I>,
                    sucess_tokens: TokenStream,
                    error_enum_name: &Ident,
                    enum_varient: &Ident,
                    varient_parse_error: &Ident,
                ) -> TokenStream {
                    if let Some((i, route)) = s.next() {
                        let children = print_route_segment(
                            s,
                            sucess_tokens,
                            error_enum_name,
                            enum_varient,
                            varient_parse_error,
                        );

                        route.try_parse(
                            i,
                            error_enum_name,
                            enum_varient,
                            varient_parse_error,
                            children,
                        )
                    } else {
                        quote! {
                            #sucess_tokens
                        }
                    }
                }

                let construct_variant = route.construct(enum_name);
                let parse_query = route.parse_query();

                let insure_not_trailing = route
                    .route_segments
                    .last()
                    .map(|seg| !matches!(seg, RouteSegment::CatchAll(_, _)))
                    .unwrap_or(true);

                print_route_segment(
                    route_segments.peekable(),
                    return_constructed(
                        insure_not_trailing,
                        construct_variant,
                        &error_enum_name,
                        enum_varient,
                        &varient_parse_error,
                        parse_query,
                    ),
                    &error_enum_name,
                    enum_varient,
                    &varient_parse_error,
                )
            }
            Self::StaticEnd(route) => {
                let varient_parse_error = route.error_ident();
                let enum_varient = &route.route_name;
                let construct_variant = route.construct(enum_name);
                let parse_query = route.parse_query();

                return_constructed(
                    true,
                    construct_variant,
                    &error_enum_name,
                    enum_varient,
                    &varient_parse_error,
                    parse_query,
                )
            }
        }
    }
}

fn return_constructed(
    insure_not_trailing: bool,
    construct_variant: TokenStream,
    error_enum_name: &Ident,
    enum_varient: &Ident,
    varient_parse_error: &Ident,
    parse_query: TokenStream,
) -> TokenStream {
    if insure_not_trailing {
        quote! {
            let remaining_segments = segments.clone();
            let mut segments_clone = segments.clone();
            let next_segment = segments_clone.next();
            let segment_after_next = segments_clone.next();
            match (next_segment, segment_after_next) {
                // This is the last segment, return the parsed route
                (None, _) | (Some(""), None) => {
                    #parse_query
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
    } else {
        quote! {
            #parse_query
            return Ok(#construct_variant);
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
