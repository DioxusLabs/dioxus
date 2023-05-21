use proc_macro2::TokenStream;
use quote::quote;
use slab::Slab;
use syn::Ident;

use crate::{
    nest::Layout,
    route::Route,
    segment::{static_segment_idx, RouteSegment},
};

#[derive(Debug, Clone, Default)]
pub struct RouteTree<'a> {
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
                RouteTreeSegmentData::Layout { .. } => 1,
                RouteTreeSegmentData::Route(route) => {
                    // Routes that end in a catch all segment should be checked last
                    match route.segments.last() {
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
            RouteTreeSegmentData::Layout { children, .. } => children.clone(),
            _ => Vec::new(),
        }
    }

    fn try_children_mut(&mut self, element: usize) -> Option<&mut Vec<usize>> {
        let element = self.entries.get_mut(element).unwrap();
        match element {
            RouteTreeSegmentData::Static { children, .. } => Some(children),
            RouteTreeSegmentData::Layout { children, .. } => Some(children),
            _ => None,
        }
    }

    fn children_mut(&mut self, element: usize) -> &mut Vec<usize> {
        self.try_children_mut(element)
            .expect("Cannot get children of non static or layout segment")
    }

    pub fn new(routes: &'a [Route], layouts: &'a [Layout]) -> Self {
        let routes = routes
            .iter()
            .map(|route| RouteIter::new(route, layouts))
            .collect::<Vec<_>>();

        let mut myself = Self::default();
        myself.roots = myself.construct(routes);
        myself.sort_children();

        myself
    }

    pub fn construct(&mut self, routes: Vec<RouteIter<'a>>) -> Vec<usize> {
        let mut segments = Vec::new();

        // Add all routes to the tree
        for mut route in routes {
            let mut current_route: Option<usize> = None;

            // First add a layout if there is one
            while let Some(layout) = route.next_layout() {
                let segments_iter: std::slice::Iter<RouteSegment> = layout.segments.iter();

                // Add all static segments of the layout
                'o: for (index, segment) in segments_iter.enumerate() {
                    match segment {
                        RouteSegment::Static(segment) => {
                            // Check if the segment already exists
                            {
                                // Either look for the segment in the current route or in the static segments
                                let segments = current_route
                                    .map(|id| self.children(id))
                                    .unwrap_or_else(|| segments.clone());

                                for seg in segments.iter() {
                                    let seg = self.get(*seg).unwrap();
                                    if let RouteTreeSegmentData::Static {
                                        segment: s,
                                        children,
                                        ..
                                    } = seg
                                    {
                                        if s == segment {
                                            // If it does, just update the current route
                                            current_route = children.last().cloned();
                                            continue 'o;
                                        }
                                    }
                                }
                            }

                            let static_segment = RouteTreeSegmentData::Static {
                                segment,
                                children: Vec::new(),
                                error_variant: route.error_variant(),
                                index,
                            };

                            // If it doesn't, add the segment to the current route
                            let static_segment = self.entries.insert(static_segment);

                            let current_children = current_route
                                .map(|id| self.children_mut(id))
                                .unwrap_or_else(|| &mut segments);
                            current_children.push(static_segment);
                        }
                        // If there is a dynamic segment, stop adding static segments
                        RouteSegment::Dynamic(..) => break,
                        RouteSegment::CatchAll(..) => {
                            todo!("Catch all segments are not allowed in layouts")
                        }
                    }
                }

                // Add the layout to the current route
                let layout = RouteTreeSegmentData::Layout {
                    layout,
                    children: Vec::new(),
                };

                let layout = self.entries.insert(layout);
                let segments = match current_route.and_then(|id| self.get_mut(id)) {
                    Some(RouteTreeSegmentData::Static { children, .. }) => children,
                    Some(_) => unreachable!(),
                    None => &mut segments,
                };
                segments.push(layout);

                // Update the current route
                current_route = segments.last().cloned();
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
                            self.children_mut(id).extend(new_children.into_iter());
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
                    let id = self
                        .entries
                        .insert(RouteTreeSegmentData::Route(route.route));
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
pub enum RouteTreeSegmentData<'a> {
    Static {
        segment: &'a str,
        error_variant: StaticErrorVariant,
        index: usize,
        children: Vec<usize>,
    },
    Layout {
        layout: &'a Layout,
        children: Vec<usize>,
    },
    Route(&'a Route),
}

impl<'a> RouteTreeSegmentData<'a> {
    pub fn to_tokens(
        &self,
        tree: &RouteTree,
        enum_name: syn::Ident,
        error_enum_name: syn::Ident,
        layouts: &[Layout],
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
                let error_ident = static_segment_idx(*index);

                let children = children.iter().map(|child| {
                    let child = tree.get(*child).unwrap();
                    child.to_tokens(tree, enum_name.clone(), error_enum_name.clone(), layouts)
                });

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
            RouteTreeSegmentData::Route(route) => {
                // At this point, we have matched all static segments, so we can just check if the remaining segments match the route
                let varient_parse_error = route.error_ident();
                let enum_varient = &route.route_name;

                let route_segments = route
                    .segments
                    .iter()
                    .enumerate()
                    .skip_while(|(_, seg)| matches!(seg, RouteSegment::Static(_)));

                let construct_variant = route.construct(enum_name, layouts);
                let parse_query = route.parse_query();

                let insure_not_trailing = route
                    .segments
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
            Self::Layout { layout, children } => {
                // At this point, we have matched all static segments, so we can just check if the remaining segments match the route
                let varient_parse_error: Ident = layout.error_ident();
                let enum_varient = &layout.layout_name;

                let route_segments = layout
                    .segments
                    .iter()
                    .enumerate()
                    .skip_while(|(_, seg)| matches!(seg, RouteSegment::Static(_)));

                let parse_children = children
                    .iter()
                    .map(|child| {
                        let child = tree.get(*child).unwrap();
                        child.to_tokens(tree, enum_name.clone(), error_enum_name.clone(), layouts)
                    })
                    .collect();

                print_route_segment(
                    route_segments.peekable(),
                    parse_children,
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

pub struct RouteIter<'a> {
    route: &'a Route,
    layouts: &'a [Layout],
    layout_index: usize,
    static_segment_index: usize,
}

impl<'a> RouteIter<'a> {
    fn new(route: &'a Route, layouts: &'a [Layout]) -> Self {
        Self {
            route,
            layouts,
            layout_index: 0,
            static_segment_index: 0,
        }
    }

    fn next_layout(&mut self) -> Option<&'a Layout> {
        let idx = self.layout_index;
        let layout_index = self.route.layouts.get(idx)?;
        let layout = &self.layouts[layout_index.0];
        self.layout_index += 1;
        Some(layout)
    }

    fn next_static_segment(&mut self) -> Option<(usize, &'a str)> {
        let idx = self.static_segment_index;
        let segment = self.route.segments.get(idx)?;
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
            varient_parse_error: self.route.error_ident(),
            enum_varient: self.route.route_name.clone(),
        }
    }
}
