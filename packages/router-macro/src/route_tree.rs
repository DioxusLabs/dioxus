use proc_macro2::TokenStream;
use quote::quote;
use slab::Slab;
use syn::Ident;

use crate::{
    RouteEndpoint,
    nest::{Nest, NestId},
    redirect::Redirect,
    route::{Route, RouteType},
    segment::RouteSegment,
};

#[derive(Debug, Clone, Default)]
pub(crate) struct ParseRouteTree<'a> {
    pub roots: Vec<usize>,
    entries: Slab<RouteTreeSegmentData<'a>>,
}

impl<'a> ParseRouteTree<'a> {
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

    pub(crate) fn new(endpoints: &'a [RouteEndpoint], nests: &'a [Nest]) -> Self {
        let routes = endpoints
            .iter()
            .map(|endpoint| match endpoint {
                RouteEndpoint::Route(route) => PathIter::new_route(route, nests),
                RouteEndpoint::Redirect(redirect) => PathIter::new_redirect(redirect, nests),
            })
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
                'o: for segment in segments_iter {
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
                                    if let RouteTreeSegmentData::Static { segment: s, .. } = seg
                                        && *s == segment
                                    {
                                        // If it does, just update the current route
                                        current_route = Some(seg_id);
                                        continue 'o;
                                    }
                                }
                            }

                            let static_segment = RouteTreeSegmentData::Static {
                                segment,
                                children: Vec::new(),
                                site: nest.site_ident(),
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
                            unimplemented!("Catch all segments are not allowed in nests")
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
                Some(segment) => {
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
                                site: route.site.clone(),
                                children: self.construct(vec![route]),
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

// First deduplicate the routes by the static part of the route
#[derive(Debug, Clone)]
pub(crate) enum RouteTreeSegmentData<'a> {
    Static {
        segment: &'a str,
        /// The `RouteMatchSite` const a mismatch is reported against: the first route (or
        /// nest) that introduced this segment into the tree.
        site: Ident,
        children: Vec<usize>,
    },
    Nest {
        nest: &'a Nest,
        children: Vec<usize>,
    },
    Route(&'a Route),
    Redirect(&'a Redirect),
}

impl RouteTreeSegmentData<'_> {
    /// Emit the parser for this node. `depth` is the index into `__segments` of the next
    /// segment to consume; every static or dynamic segment matched on the way here consumed one.
    pub fn to_tokens(
        &self,
        nests: &[Nest],
        tree: &ParseRouteTree,
        enum_name: syn::Ident,
        depth: usize,
    ) -> TokenStream {
        match self {
            RouteTreeSegmentData::Static {
                segment,
                children,
                site,
                ..
            } => {
                if segment.is_empty() {
                    let children = children.iter().map(|child| {
                        let child = tree.get(*child).unwrap();
                        child.to_tokens(nests, tree, enum_name.clone(), depth)
                    });
                    return quote! {
                        {
                            #(#children)*
                        }
                    };
                }

                let children = children.iter().map(|child| {
                    let child = tree.get(*child).unwrap();
                    child.to_tokens(nests, tree, enum_name.clone(), depth + 1)
                });

                quote! {
                    if let Some(__segment) = __segments.get(#depth) {
                        if __segment == #segment {
                            #(#children)*
                        } else {
                            __errors.push(#site.static_segment(#segment, __segment));
                        }
                    }
                }
            }
            RouteTreeSegmentData::Route(route) => {
                // At this point, we have matched all static segments, so we can just check if the remaining segments match the route
                let site = route.site_ident();

                let route_segments = route
                    .segments
                    .iter()
                    .enumerate()
                    .skip_while(|(_, seg)| matches!(seg, RouteSegment::Static(_)))
                    .filter(|(i, _)| {
                        // Don't add any trailing static segments. We strip them during parsing so that routes can accept either `/route/` and `/route`
                        !is_trailing_static_segment(&route.segments, *i)
                    })
                    .map(|(_, seg)| seg)
                    .collect::<Vec<_>>();

                let construct_variant = route.construct(nests, enum_name);
                let parse_query = route.parse_query();
                let parse_hash = route.parse_hash();

                let insure_not_trailing = match route.ty {
                    RouteType::Leaf { .. } => route
                        .segments
                        .last()
                        .map(|seg| !matches!(seg, RouteSegment::CatchAll(_, _)))
                        .unwrap_or(true),
                    RouteType::Child(_) => false,
                };

                let print_route_segment = print_route_segment(
                    &route_segments,
                    depth,
                    &|depth| {
                        return_constructed(
                            insure_not_trailing,
                            depth,
                            construct_variant.clone(),
                            &site,
                            parse_query.clone(),
                            parse_hash.clone(),
                        )
                    },
                    &site,
                );

                match &route.ty {
                    RouteType::Child(child) => {
                        let ty = &child.ty;
                        let child_name = &child.ident;

                        quote! {
                            match __segments.child::<#ty>(#depth, raw_query, raw_hash, #site) {
                                Ok(#child_name) => {
                                    #print_route_segment
                                }
                                Err(err) => {
                                    __errors.push(err);
                                }
                            }
                        }
                    }
                    RouteType::Leaf { .. } => print_route_segment,
                }
            }
            Self::Nest { nest, children } => {
                // At this point, we have matched all static segments, so we can just check if the remaining segments match the route
                let site = nest.site_ident();

                let route_segments = nest
                    .segments
                    .iter()
                    .skip_while(|seg| matches!(seg, RouteSegment::Static(_)))
                    .collect::<Vec<_>>();

                print_route_segment(
                    &route_segments,
                    depth,
                    &|depth| {
                        children
                            .iter()
                            .map(|child| {
                                let child = tree.get(*child).unwrap();
                                child.to_tokens(nests, tree, enum_name.clone(), depth)
                            })
                            .collect()
                    },
                    &site,
                )
            }
            Self::Redirect(redirect) => {
                // At this point, we have matched all static segments, so we can just check if the remaining segments match the route
                let site = redirect.site_ident();

                let route_segments = redirect
                    .segments
                    .iter()
                    .skip_while(|seg| matches!(seg, RouteSegment::Static(_)))
                    .collect::<Vec<_>>();

                let parse_query = redirect.parse_query();
                let parse_hash = redirect.parse_hash();

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
                    &route_segments,
                    depth,
                    &|depth| {
                        return_constructed(
                            insure_not_trailing,
                            depth,
                            return_redirect.clone(),
                            &site,
                            parse_query.clone(),
                            parse_hash.clone(),
                        )
                    },
                    &site,
                )
            }
        }
    }
}

/// Nest the parsers for `segments` starting at `depth`, with `success` emitted innermost at the
/// depth reached once every segment matched.
fn print_route_segment(
    segments: &[&RouteSegment],
    depth: usize,
    success: &dyn Fn(usize) -> TokenStream,
    site: &Ident,
) -> TokenStream {
    match segments.split_first() {
        Some((segment, rest)) => {
            let children = print_route_segment(rest, depth + 1, success, site);
            segment.try_parse(depth, site, children)
        }
        None => success(depth),
    }
}

fn return_constructed(
    insure_not_trailing: bool,
    depth: usize,
    construct_variant: TokenStream,
    site: &Ident,
    parse_query: TokenStream,
    parse_hash: TokenStream,
) -> TokenStream {
    if insure_not_trailing {
        quote! {
            if __segments.ends_at(#depth) {
                #parse_query
                #parse_hash
                return Ok(#construct_variant);
            } else {
                __errors.push(#site.extra_segments(__segments.trailing(#depth)));
            }
        }
    } else {
        quote! {
            #parse_query
            #parse_hash
            return Ok(#construct_variant);
        }
    }
}

pub struct PathIter<'a> {
    final_segment: RouteTreeSegmentData<'a>,
    active_nests: &'a [NestId],
    all_nests: &'a [Nest],
    segments: &'a [RouteSegment],
    site: Ident,
    nest_index: usize,
    static_segment_index: usize,
}

impl<'a> PathIter<'a> {
    fn new_route(route: &'a Route, nests: &'a [Nest]) -> Self {
        Self {
            final_segment: RouteTreeSegmentData::Route(route),
            active_nests: &*route.nests,
            segments: &*route.segments,
            site: route.site_ident(),
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
            site: redirect.site_ident(),
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

    fn next_static_segment(&mut self) -> Option<&'a str> {
        let idx = self.static_segment_index;
        let segment = self.segments.get(idx)?;
        // Don't add any trailing static segments. We strip them during parsing so that routes can accept either `/route/` and `/route`
        if is_trailing_static_segment(self.segments, idx) {
            return None;
        }
        match segment {
            RouteSegment::Static(segment) => {
                self.static_segment_index += 1;
                Some(segment)
            }
            _ => None,
        }
    }
}

// If this is the last segment and it is an empty trailing segment, skip parsing it. The parsing code handles parsing /path/ and /path
pub(crate) fn is_trailing_static_segment(segments: &[RouteSegment], index: usize) -> bool {
    // This can only be a trailing segment if we have more than one segment and this is the last segment
    matches!(segments.get(index), Some(RouteSegment::Static(segment)) if segment.is_empty() && index == segments.len() - 1 && segments.len() > 1)
}
