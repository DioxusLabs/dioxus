use quote::{format_ident, quote};
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

    pub fn write_segment(&self) -> TokenStream2 {
        match self {
            Self::Static(segment) => quote! { write!(f, "/{}", #segment)?; },
            Self::Dynamic(ident, _) => quote! {
                {
                    let as_string = #ident.to_string();
                    write!(f, "/{}", dioxus_router::exports::urlencoding::encode(&as_string))?;
                }
            },
            Self::CatchAll(ident, _) => quote! { #ident.display_route_segments(f)?; },
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
        error_enum_variant: &Ident,
        inner_parse_enum: &Ident,
        parse_children: TokenStream2,
    ) -> TokenStream2 {
        let error_name = self.error_name(idx);
        match self {
            Self::Static(segment) => {
                quote! {
                    {
                        let mut segments = segments.clone();
                        let segment = segments.next();
                        let segment = segment.as_deref();
                        let parsed = if let Some(#segment) = segment {
                            Ok(())
                        } else {
                            Err(#error_enum_name::#error_enum_variant(#inner_parse_enum::#error_name(segment.map(|s|s.to_string()).unwrap_or_default())))
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
                        let segment = segments.next();
                        let parsed = if let Some(segment) = segment.as_deref() {
                            <#ty as dioxus_router::routable::FromRouteSegment>::from_route_segment(segment).map_err(|err| #error_enum_name::#error_enum_variant(#inner_parse_enum::#error_name(err)))
                        } else {
                            Err(#error_enum_name::#error_enum_variant(#inner_parse_enum::#missing_error_name))
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
                            let remaining_segments: Vec<_> = segments.collect();
                            let mut new_segments: Vec<&str> = Vec::new();
                            for segment in &remaining_segments {
                                new_segments.push(&*segment);
                            }
                            <#ty as dioxus_router::routable::FromRouteSegments>::from_route_segments(&new_segments).map_err(|err| #error_enum_name::#error_enum_variant(#inner_parse_enum::#error_name(err)))
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

pub(crate) fn create_error_type(
    error_name: Ident,
    segments: &[RouteSegment],
    child_type: Option<&Type>,
) -> TokenStream2 {
    let mut error_variants = Vec::new();
    let mut display_match = Vec::new();

    for (i, segment) in segments.iter().enumerate() {
        let error_name = segment.error_name(i);
        match segment {
            RouteSegment::Static(index) => {
                error_variants.push(quote! { #error_name(String) });
                display_match.push(quote! { Self::#error_name(found) => write!(f, "Static segment '{}' did not match instead found '{}'", #index, found)? });
            }
            RouteSegment::Dynamic(ident, ty) => {
                let missing_error = segment.missing_error_name().unwrap();
                error_variants.push(
                    quote! { #error_name(<#ty as dioxus_router::routable::FromRouteSegment>::Err) },
                );
                display_match.push(quote! { Self::#error_name(err) => write!(f, "Dynamic segment '({}:{})' did not match: {}", stringify!(#ident), stringify!(#ty), err)? });
                error_variants.push(quote! { #missing_error });
                display_match.push(quote! { Self::#missing_error => write!(f, "Dynamic segment '({}:{})' was missing", stringify!(#ident), stringify!(#ty))? });
            }
            RouteSegment::CatchAll(ident, ty) => {
                error_variants.push(quote! { #error_name(<#ty as dioxus_router::routable::FromRouteSegments>::Err) });
                display_match.push(quote! { Self::#error_name(err) => write!(f, "Catch-all segment '({}:{})' did not match: {}", stringify!(#ident), stringify!(#ty), err)? });
            }
        }
    }

    let child_type_variant = child_type
        .map(|child_type| {
            quote! { ChildRoute(<#child_type as std::str::FromStr>::Err) }
        })
        .into_iter();

    let child_type_error = child_type
        .map(|_| {
            quote! {
                Self::ChildRoute(error) => {
                    write!(f, "{}", error)?
                }
            }
        })
        .into_iter();

    quote! {
        #[allow(non_camel_case_types)]
        #[allow(clippy::derive_partial_eq_without_eq)]
        pub enum #error_name {
            ExtraSegments(String),
            #(#child_type_variant,)*
            #(#error_variants,)*
        }

        impl std::fmt::Debug for #error_name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}({})", stringify!(#error_name), self)
            }
        }

        impl std::fmt::Display for #error_name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    Self::ExtraSegments(segments) => {
                        write!(f, "Found additional trailing segments: {}", segments)?
                    },
                    #(#child_type_error,)*
                    #(#display_match,)*
                }
                Ok(())
            }
        }
    }
}
