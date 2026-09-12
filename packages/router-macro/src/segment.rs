use quote::quote;
use syn::{Ident, Type};

use proc_macro2::{Span, TokenStream as TokenStream2};

use crate::{hash::HashFragment, query::QuerySegment};

#[derive(Debug, Clone)]
pub enum RouteSegment {
    Static(String),
    Dynamic(Ident, Type),
    CatchAll(Ident, Type),
}

impl RouteSegment {
    pub fn name(&self) -> Option<Ident> {
        match self {
            Self::Static(_) => None,
            Self::Dynamic(ident, _) => Some(ident.clone()),
            Self::CatchAll(ident, _) => Some(ident.clone()),
        }
    }

    /// Emit the parser for this segment at `depth`, running `parse_children` once it matched.
    ///
    /// `site` names the `RouteMatchSite` const failures are reported against.
    pub fn try_parse(
        &self,
        depth: usize,
        site: &Ident,
        parse_children: TokenStream2,
    ) -> TokenStream2 {
        match self {
            Self::Static(segment) => {
                quote! {
                    if let Some(#segment) = __segments.get(#depth) {
                        #parse_children
                    } else {
                        __errors.push(#site.static_segment(#segment, __segments.get(#depth).unwrap_or_default()));
                    }
                }
            }
            Self::Dynamic(name, ty) => {
                quote! {
                    match __segments.dynamic::<#ty>(#depth, #site, stringify!(#name), stringify!(#ty)) {
                        Ok(#name) => {
                            #parse_children
                        }
                        Err(err) => {
                            __errors.push(err);
                        }
                    }
                }
            }
            Self::CatchAll(name, ty) => {
                quote! {
                    match __segments.catch_all::<#ty>(#depth, #site, stringify!(#name), stringify!(#ty)) {
                        Ok(#name) => {
                            #parse_children
                        }
                        Err(err) => {
                            __errors.push(err);
                        }
                    }
                }
            }
        }
    }
}

/// Emit the `Display` code for a run of segments, writing to the `fmt::Write` bound to `f`.
///
/// Consecutive static segments collapse into one `write_str` of their joined text.
pub fn write_segments<'a>(segments: impl IntoIterator<Item = &'a RouteSegment>) -> TokenStream2 {
    let mut tokens = TokenStream2::new();
    let mut pending = String::new();
    let flush = |pending: &mut String, tokens: &mut TokenStream2| {
        if !pending.is_empty() {
            tokens.extend(quote! { f.write_str(#pending)?; });
            pending.clear();
        }
    };

    for segment in segments {
        match segment {
            RouteSegment::Static(segment) => {
                pending.push('/');
                pending.push_str(segment);
            }
            RouteSegment::Dynamic(ident, _) => {
                flush(&mut pending, &mut tokens);
                tokens.extend(quote! {
                    dioxus_router::route_match::write_path_segment(f, &#ident)?;
                });
            }
            RouteSegment::CatchAll(ident, _) => {
                flush(&mut pending, &mut tokens);
                tokens.extend(quote! {
                    dioxus_router::ToRouteSegments::display_route_segments(#ident, f)?;
                });
            }
        }
    }
    flush(&mut pending, &mut tokens);
    tokens
}

/// Emit the `RouteMatchSite` const named `site` for one route, redirect or nest.
pub fn site_const(
    site: &Ident,
    error_type: &Ident,
    kind: &str,
    name: &str,
    route: &str,
) -> TokenStream2 {
    let error_type = error_type.to_string();
    quote! {
        #[allow(non_upper_case_globals)]
        const #site: &dioxus_router::route_match::RouteMatchSite =
            &dioxus_router::route_match::RouteMatchSite {
                error_type: #error_type,
                kind: #kind,
                name: #name,
                route: #route,
            };
    }
}

pub fn parse_route_segments<'a>(
    route_span: Span,
    fields: impl Iterator<Item = (&'a Ident, &'a Type)> + Clone,
    route: &str,
) -> syn::Result<(
    Vec<RouteSegment>,
    Option<QuerySegment>,
    Option<HashFragment>,
)> {
    let mut route_segments = Vec::new();

    let (route_string, hash) = match route.rsplit_once('#') {
        Some((route, hash)) => (
            route,
            Some(HashFragment::parse_from_str(
                route_span,
                fields.clone(),
                hash,
            )?),
        ),
        None => (route, None),
    };

    let (route_string, query) = match route_string.rsplit_once('?') {
        Some((route, query)) => (
            route,
            Some(QuerySegment::parse_from_str(
                route_span,
                fields.clone(),
                query,
            )?),
        ),
        None => (route_string, None),
    };
    let mut iterator = route_string.split('/');

    // skip the first empty segment
    let first = iterator.next();
    if first != Some("") {
        return Err(syn::Error::new(
            route_span,
            format!(
                "Routes should start with /. Error found in the route '{}'",
                route
            ),
        ));
    }

    while let Some(segment) = iterator.next() {
        if let Some(segment) = segment.strip_prefix(':') {
            let spread = segment.starts_with("..");

            let ident = if spread {
                segment[2..].to_string()
            } else {
                segment.to_string()
            };

            let field = fields.clone().find(|(name, _)| **name == ident);

            let ty = if let Some(field) = field {
                field.1.clone()
            } else {
                return Err(syn::Error::new(
                    route_span,
                    format!("Could not find a field with the name '{}'", ident,),
                ));
            };
            if spread {
                route_segments.push(RouteSegment::CatchAll(
                    Ident::new(&ident, Span::call_site()),
                    ty,
                ));

                if iterator.next().is_some() {
                    return Err(syn::Error::new(
                        route_span,
                        "Catch-all route segments must be the last segment in a route. The route segments after the catch-all segment will never be matched.",
                    ));
                } else {
                    break;
                }
            } else {
                route_segments.push(RouteSegment::Dynamic(
                    Ident::new(&ident, Span::call_site()),
                    ty,
                ));
            }
        } else {
            route_segments.push(RouteSegment::Static(segment.to_string()));
        }
    }

    Ok((route_segments, query, hash))
}
