use quote::{format_ident, quote};
use syn::{Ident, Type};

use proc_macro2::{Span, TokenStream as TokenStream2};

use crate::query::QuerySegment;

#[derive(Debug)]
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

    pub fn ty(&self) -> Option<&Type> {
        match self {
            Self::Static(_) => None,
            Self::Dynamic(_, ty) => Some(ty),
            Self::CatchAll(_, ty) => Some(ty),
        }
    }

    pub fn write_segment(&self) -> TokenStream2 {
        match self {
            Self::Static(segment) => quote! { write!(f, "/{}", #segment)?; },
            Self::Dynamic(ident, _) => quote! { write!(f, "/{}", #ident)?; },
            Self::CatchAll(ident, _) => quote! { #ident.display_route_segements(f)?; },
        }
    }

    pub fn error_name(&self, idx: usize) -> Ident {
        match self {
            Self::Static(_) => static_segment_idx(idx),
            Self::Dynamic(ident, _) => format_ident!("{}ParseError", ident),
            Self::CatchAll(ident, _) => format_ident!("{}ParseError", ident),
        }
    }

    pub fn missing_error_name(&self) -> Option<Ident> {
        match self {
            Self::Dynamic(ident, _) => Some(format_ident!("{}MissingError", ident)),
            _ => None,
        }
    }

    pub fn try_parse(
        &self,
        idx: usize,
        error_enum_name: &Ident,
        error_enum_varient: &Ident,
        inner_parse_enum: &Ident,
        parse_children: TokenStream2,
    ) -> TokenStream2 {
        let error_name = self.error_name(idx);
        match self {
            Self::Static(segment) => {
                quote! {
                    {
                        let mut segments = segments.clone();
                        let parsed = if let Some(#segment) = segments.next() {
                            Ok(())
                        } else {
                            Err(#error_enum_name::#error_enum_varient(#inner_parse_enum::#error_name))
                        };
                        match parsed {
                            Ok(_) => {
                                #parse_children
                            }
                            Err(err) => {
                                errors.push(err);
                            }
                        }
                    }
                }
            }
            Self::Dynamic(name, ty) => {
                let missing_error_name = self.missing_error_name().unwrap();
                quote! {
                    {
                        let mut segments = segments.clone();
                        let parsed = if let Some(segment) = segments.next() {
                            <#ty as dioxus_router_core::router::FromRouteSegment>::from_route_segment(segment).map_err(|err| #error_enum_name::#error_enum_varient(#inner_parse_enum::#error_name(err)))
                        } else {
                            Err(#error_enum_name::#error_enum_varient(#inner_parse_enum::#missing_error_name))
                        };
                        match parsed {
                            Ok(#name) => {
                                #parse_children
                            }
                            Err(err) => {
                                errors.push(err);
                            }
                        }
                    }
                }
            }
            Self::CatchAll(name, ty) => {
                quote! {
                    {
                        let parsed = {
                            let mut segments = segments.clone();
                            let segments: Vec<_> = segments.collect();
                            <#ty as dioxus_router_core::router::FromRouteSegments>::from_route_segments(&segments).map_err(|err| #error_enum_name::#error_enum_varient(#inner_parse_enum::#error_name(err)))
                        };
                        match parsed {
                            Ok(#name) => {
                                #parse_children
                            }
                            Err(err) => {
                                errors.push(err);
                            }
                        }
                    }
                }
            }
        }
    }
}

pub fn static_segment_idx(idx: usize) -> Ident {
    format_ident!("StaticSegment{}ParseError", idx)
}

pub fn parse_route_segments(
    route_name: &Ident,
    fields: &syn::FieldsNamed,
    route: &str,
) -> syn::Result<(Vec<RouteSegment>, Option<QuerySegment>)> {
    let mut route_segments = Vec::new();

    let (route_string, query) = match route.rsplit_once('?') {
        Some((route, query)) => (route, Some(query)),
        None => (route, None),
    };
    let mut iterator = route_string.split('/');

    // skip the first empty segment
    let first = iterator.next();
    if first != Some("") {
        return Err(syn::Error::new_spanned(
            route_name,
            format!(
                "Routes should start with /. Error found in the route '{}'",
                route
            ),
        ));
    }

    while let Some(segment) = iterator.next() {
        if let Some(segment) = segment.strip_prefix(':') {
            let spread = segment.starts_with("...");

            let ident = if spread {
                segment[3..].to_string()
            } else {
                segment.to_string()
            };

            let field = fields.named.iter().find(|field| match field.ident {
                Some(ref field_ident) => *field_ident == ident,
                None => false,
            });

            let ty = if let Some(field) = field {
                field.ty.clone()
            } else {
                return Err(syn::Error::new_spanned(
                    route_name,
                    format!(
                        "Could not find a field with the name '{}' in the variant '{}'",
                        ident, route_name
                    ),
                ));
            };
            if spread {
                route_segments.push(RouteSegment::CatchAll(
                    Ident::new(&ident, Span::call_site()),
                    ty,
                ));

                if iterator.next().is_some() {
                    return Err(syn::Error::new_spanned(
                        route,
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

    // check if the route has a query string
    let parsed_query = match query {
        Some(query) => {
            if let Some(query) = query.strip_prefix(':') {
                let query_ident = Ident::new(query, Span::call_site());
                let field = fields.named.iter().find(|field| match field.ident {
                    Some(ref field_ident) => field_ident == &query_ident,
                    None => false,
                });

                let ty = if let Some(field) = field {
                    field.ty.clone()
                } else {
                    return Err(syn::Error::new_spanned(
                        route_name,
                        format!(
                            "Could not find a field with the name '{}' in the variant '{}'",
                            query_ident, route_name
                        ),
                    ));
                };

                Some(QuerySegment {
                    ident: query_ident,
                    ty,
                })
            } else {
                None
            }
        }
        None => None,
    };

    Ok((route_segments, parsed_query))
}
