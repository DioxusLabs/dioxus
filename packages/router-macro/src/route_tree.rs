use proc_macro2::TokenStream;
use quote::quote;
use slab::Slab;
use syn::Ident;

use crate::{
    nest::{Nest, NestId},
    redirect::Redirect,
    route::{Route, RouteType},
    segment::{static_segment_idx, RouteSegment},
};

#[derive(Debug, Clone, Default)]
pub(crate) struct RouteTree<'a> {
    pub roots: Vec<usize>,
    entries: Slab<RouteTreeSegmentData<'a>>,
}

impl<'a> RouteTree<'a> {
    pub fn get(&self, index: usize) -> Option<&RouteTreeSegmentData<'a>> {
        self.entries.get(index)
    }

    pub fn get_mut(&mut self, element: usize) -> Option<&mut RouteTreeSegmentData<'a>> {
        self.entries.get_mut(element)
    }

    fn sort_children(&mut self) {
        let mut old_roots = self.roots.clone();
        self.sort_ids(&mut old_roots);
        self.roots = old_roots;

        for id in self.roots.clone() {
            self.sort_children_of_id(id);
        }
    }

    fn sort_ids(&self, ids: &mut [usize]) {
        ids.sort_by_key(|&seg| {
            let seg = self.get(seg).unwrap();
            match seg {
                RouteTreeSegmentData::Static { .. } => 0,
                RouteTreeSegmentData::Nest { .. } => 1,
                RouteTreeSegmentData::Route(route) => {
                    // Routes that end in a catch all segment should be checked last
                    match route.segments.last() {
                        Some(RouteSegment::CatchAll(..)) => 2,
                        _ => 1,
                    }
                }
                RouteTreeSegmentData::Redirect(redirect) => {
                    // Routes that end in a catch all segment should be checked last
                    match redirect.segments.last() {
                        Some(RouteSegment::CatchAll(..)) => 2,
                        _ => 1,
                    }
                }
            }
        });
    }

    fn sort_children_of_id(&mut self, id: usize) {
        // Sort segments so that all static routes are checked before dynamic routes
        let mut children = self.children(id);

        self.sort_ids(&mut children);

        if let Some(old) = self.try_children_mut(id) {
            old.clone_from(&children)
        }

        for id in children {
            self.sort_children_of_id(id);
        }
    }

    fn children(&self, element: usize) -> Vec<usize> {
        let element = self.entries.get(element).unwrap();
        match element {
            RouteTreeSegmentData::Static { children, .. } => children.clone(),
            RouteTreeSegmentData::Nest { children, .. } => children.clone(),
            _ => Vec::new(),
        }
    }

    fn try_children_mut(&mut self, element: usize) -> Option<&mut Vec<usize>> {
        let element = self.entries.get_mut(element).unwrap();
        match element {
            RouteTreeSegmentData::Static { children, .. } => Some(children),
            RouteTreeSegmentData::Nest { children, .. } => Some(children),
            _ => None,
        }
    }

    fn children_mut(&mut self, element: usize) -> &mut Vec<usize> {
        self.try_children_mut(element)
            .expect("Cannot get children of non static or nest segment")
    }

    pub(crate) fn new(routes: &'a [Route], nests: &'a [Nest], redirects: &'a [Redirect]) -> Self {
        let routes = routes
            .iter()
            .map(|route| PathIter::new_route(route, nests))
            .chain(
                redirects
                    .iter()
                    .map(|redirect| PathIter::new_redirect(redirect, nests)),
            )
            .collect::<Vec<_>>();

        let mut myself = Self::default();
        myself.roots = myself.construct(routes);
        myself.sort_children();

        myself
    }

    pub fn construct(&mut self, routes: Vec<PathIter<'a>>) -> Vec<usize> {
        let mut segments = Vec::new();

        // Add all routes to the tree
        for mut route in routes {
            let mut current_route: Option<usize> = None;

            // First add all nests
            while let Some(nest) = route.next_nest() {
                let segments_iter = nest.segments.iter();

                // Add all static segments of the nest
                'o: for (index, segment) in segments_iter.enumerate() {
                    match segment {
                        RouteSegment::Static(segment) => {
                            // Check if the segment already exists
                            {
                                // Either look for the segment in the current route or in the static segments
                                let segments = current_route
                                    .map(|id| self.children(id))
                                    .unwrap_or_else(|| segments.clone());

                                for &seg_id in segments.iter() {
                                    let seg = self.get(seg_id).unwrap();
                                    if let RouteTreeSegmentData::Static { segment: s, .. } = seg {
                                        if *s == segment {
                                            // If it does, just update the current route
                                            current_route = Some(seg_id);
                                            continue 'o;
                                        }
                                    }
                                }
                            }

                            let static_segment = RouteTreeSegmentData::Static {
                                segment,
                                children: Vec::new(),
                                error_variant: StaticErrorVariant {
                                    varient_parse_error: nest.error_ident(),
                                    enum_varient: nest.error_variant(),
                                },
                                index,
                            };

                            // If it doesn't, add the segment to the current route
                            let static_segment = self.entries.insert(static_segment);

                            let current_children = current_route
                                .map(|id| self.children_mut(id))
                                .unwrap_or_else(|| &mut segments);
                            current_children.push(static_segment);

                            // Update the current route
                            current_route = Some(static_segment);
                        }
                        // If there is a dynamic segment, stop adding static segments
                        RouteSegment::Dynamic(..) => break,
                        RouteSegment::CatchAll(..) => {
                            todo!("Catch all segments are not allowed in nests")
                        }
                    }
                }

                // Add the nest to the current route
                let nest = RouteTreeSegmentData::Nest {
                    nest,
                    children: Vec::new(),
                };

                let nest = self.entries.insert(nest);
                let segments = match current_route.and_then(|id| self.get_mut(id)) {
                    Some(RouteTreeSegmentData::Static { children, .. }) => children,
                    Some(RouteTreeSegmentData::Nest { children, .. }) => children,
                    Some(r) => {
                        unreachable!("{current_route:?}\n{r:?} is not a static or nest segment",)
                    }
                    None => &mut segments,
                };
                segments.push(nest);

                // Update the current route
                current_route = Some(nest);
            }

            match route.next_static_segment() {
                // If there is a static segment, check if it already exists in the tree
                Some((i, segment)) => {
                    let current_children = current_route
                        .map(|id| self.children(id))
                        .unwrap_or_else(|| segments.clone());
                    let found = current_children.iter().find_map(|&id| {
                        let seg = self.get(id).unwrap();
                        match seg {
                            RouteTreeSegmentData::Static { segment: s, .. } => {
                                (s == &segment).then_some(id)
                            }
                            _ => None,
                        }
                    });

                    match found {
                        Some(id) => {
                            // If it exists, add the route to the children of the segment
                            let new_children = self.construct(vec![route]);
                            self.children_mut(id).extend(new_children);
                        }
                        None => {
                            // If it doesn't exist, add the route as a new segment
                            let data = RouteTreeSegmentData::Static {
                                segment,
                                error_variant: route.error_variant(),
                                children: self.construct(vec![route]),
                                index: i,
                            };
                            let id = self.entries.insert(data);
                            let current_children_mut = current_route
                                .map(|id| self.children_mut(id))
                                .unwrap_or_else(|| &mut segments);
                            current_children_mut.push(id);
                        }
                    }
                }
                // If there is no static segment, add the route to the current_route
                None => {
                    let id = self.entries.insert(route.final_segment);
                    let current_children_mut = current_route
                        .map(|id| self.children_mut(id))
                        .unwrap_or_else(|| &mut segments);
                    current_children_mut.push(id);
                }
            }
        }

        segments
    }
}

#[derive(Debug, Clone)]
pub struct StaticErrorVariant {
    varient_parse_error: Ident,
    enum_varient: Ident,
}

// First deduplicate the routes by the static part of the route
#[derive(Debug, Clone)]
pub(crate) enum RouteTreeSegmentData<'a> {
    Static {
        segment: &'a str,
        error_variant: StaticErrorVariant,
        index: usize,
        children: Vec<usize>,
    },
    Nest {
        nest: &'a Nest,
        children: Vec<usize>,
    },
    Route(&'a Route),
    Redirect(&'a Redirect),
}

impl<'a> RouteTreeSegmentData<'a> {
    pub fn to_tokens(
        &self,
        nests: &[Nest],
        tree: &RouteTree,
        enum_name: syn::Ident,
        error_enum_name: syn::Ident,
    ) -> TokenStream {
        match self {
            RouteTreeSegmentData::Static {
                segment,
                children,
                index,
                error_variant:
                    StaticErrorVariant {
                        varient_parse_error,
                        enum_varient,
                    },
            } => {
                let children = children.iter().map(|child| {
                    let child = tree.get(*child).unwrap();
                    child.to_tokens(nests, tree, enum_name.clone(), error_enum_name.clone())
                });

                if segment.is_empty() {
                    return quote! {
                        {
                            #(#children)*
                        }
                    };
                }

                let error_ident = static_segment_idx(*index);

                quote! {
                    {
                        let mut segments = segments.clone();
                        let segment = segments.next();
                        if let Some(segment) = segment.as_deref() {
                            if #segment == segment {
                                #(#children)*
                            }
                            else {
                                errors.push(#error_enum_name::#enum_varient(#varient_parse_error::#error_ident(segment.to_string())))
                            }
                        }
                    }
                }
            }
            RouteTreeSegmentData::Route(route) => {
                // At this point, we have matched all static segments, so we can just check if the remaining segments match the route
                let varient_parse_error = route.error_ident();
                let enum_varient = &route.route_name;

                let route_segments = route
                    .segments
                    .iter()
                    .enumerate()
                    .skip_while(|(_, seg)| matches!(seg, RouteSegment::Static(_)));

                let construct_variant = route.construct(nests, enum_name);
                let parse_query = route.parse_query();

                let insure_not_trailing = match route.ty {
                    RouteType::Leaf { .. } => route
                        .segments
                        .last()
                        .map(|seg| !matches!(seg, RouteSegment::CatchAll(_, _)))
                        .unwrap_or(true),
                    RouteType::Child(_) => false,
                };

                let print_route_segment = print_route_segment(
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
                );

                match &route.ty {
                    RouteType::Child(child) => {
                        let ty = &child.ty;
                        let child_name = &child.ident;

                        quote! {
                            let mut trailing = String::from("/");
                            for seg in segments.clone() {
                                trailing += &*seg;
                                trailing += "/";
                            }
                            trailing.pop();
                            match #ty::from_str(&trailing).map_err(|err| #error_enum_name::#enum_varient(#varient_parse_error::ChildRoute(err))) {
                                Ok(#child_name) => {
                                    #print_route_segment
                                }
                                Err(err) => {
                                    errors.push(err);
                                }
                            }
                        }
                    }
                    RouteType::Leaf { .. } => print_route_segment,
                }
            }
            Self::Nest { nest, children } => {
                // At this point, we have matched all static segments, so we can just check if the remaining segments match the route
                let varient_parse_error: Ident = nest.error_ident();
                let enum_varient = nest.error_variant();

                let route_segments = nest
                    .segments
                    .iter()
                    .enumerate()
                    .skip_while(|(_, seg)| matches!(seg, RouteSegment::Static(_)));

                let parse_children = children
                    .iter()
                    .map(|child| {
                        let child = tree.get(*child).unwrap();
                        child.to_tokens(nests, tree, enum_name.clone(), error_enum_name.clone())
                    })
                    .collect();

                print_route_segment(
                    route_segments.peekable(),
                    parse_children,
                    &error_enum_name,
                    &enum_varient,
                    &varient_parse_error,
                )
            }
            Self::Redirect(redirect) => {
                // At this point, we have matched all static segments, so we can just check if the remaining segments match the route
                let varient_parse_error = redirect.error_ident();
                let enum_varient = &redirect.error_variant();

                let route_segments = redirect
                    .segments
                    .iter()
                    .enumerate()
                    .skip_while(|(_, seg)| matches!(seg, RouteSegment::Static(_)));

                let parse_query = redirect.parse_query();

                let insure_not_trailing = redirect
                    .segments
                    .last()
                    .map(|seg| !matches!(seg, RouteSegment::CatchAll(_, _)))
                    .unwrap_or(true);

                let redirect_function = &redirect.function;
                let args = redirect_function.inputs.iter().map(|pat| match pat {
                    syn::Pat::Type(ident) => {
                        let name = &ident.pat;
                        quote! {#name}
                    }
                    _ => panic!("Expected closure argument to be a typed pattern"),
                });
                let return_redirect = quote! {
                    (#redirect_function)(#(#args,)*)
                };

                print_route_segment(
                    route_segments.peekable(),
                    return_constructed(
                        insure_not_trailing,
                        return_redirect,
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
        }
    }
}

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
            let next_segment = next_segment.as_deref();
            let segment_after_next = segments_clone.next();
            let segment_after_next = segment_after_next.as_deref();
            match (next_segment, segment_after_next) {
                // This is the last segment, return the parsed route
                (None, _) | (Some(""), None) => {
                    #parse_query
                    return Ok(#construct_variant);
                }
                _ => {
                    let mut trailing = String::new();
                    for seg in remaining_segments {
                        trailing += &*seg;
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

pub struct PathIter<'a> {
    final_segment: RouteTreeSegmentData<'a>,
    active_nests: &'a [NestId],
    all_nests: &'a [Nest],
    segments: &'a [RouteSegment],
    error_ident: Ident,
    error_variant: Ident,
    nest_index: usize,
    static_segment_index: usize,
}

impl<'a> PathIter<'a> {
    fn new_route(route: &'a Route, nests: &'a [Nest]) -> Self {
        Self {
            final_segment: RouteTreeSegmentData::Route(route),
            active_nests: &*route.nests,
            segments: &*route.segments,
            error_ident: route.error_ident(),
            error_variant: route.route_name.clone(),
            all_nests: nests,
            nest_index: 0,
            static_segment_index: 0,
        }
    }

    fn new_redirect(redirect: &'a Redirect, nests: &'a [Nest]) -> Self {
        Self {
            final_segment: RouteTreeSegmentData::Redirect(redirect),
            active_nests: &*redirect.nests,
            segments: &*redirect.segments,
            error_ident: redirect.error_ident(),
            error_variant: redirect.error_variant(),
            all_nests: nests,
            nest_index: 0,
            static_segment_index: 0,
        }
    }

    fn next_nest(&mut self) -> Option<&'a Nest> {
        let idx = self.nest_index;
        let nest_index = self.active_nests.get(idx)?;
        let nest = &self.all_nests[nest_index.0];
        self.nest_index += 1;
        Some(nest)
    }

    fn next_static_segment(&mut self) -> Option<(usize, &'a str)> {
        let idx = self.static_segment_index;
        let segment = self.segments.get(idx)?;
        match segment {
            RouteSegment::Static(segment) => {
                self.static_segment_index += 1;
                Some((idx, segment))
            }
            _ => None,
        }
    }

    fn error_variant(&self) -> StaticErrorVariant {
        StaticErrorVariant {
            varient_parse_error: self.error_ident.clone(),
            enum_varient: self.error_variant.clone(),
        }
    }
}
