use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Ident, LitStr};

use crate::segment::{parse_route_segments, RouteSegment};

#[derive(Debug, Clone, Copy)]
pub struct NestId(pub usize);

#[derive(Debug, Clone)]
pub struct Nest {
    pub route: String,
    pub segments: Vec<RouteSegment>,
    index: usize,
}

impl Nest {
    pub fn parse(
        input: syn::parse::ParseStream,
        children_routes: Vec<syn::FieldsNamed>,
        index: usize,
    ) -> syn::Result<Self> {
        // Parse the route
        let route: LitStr = input.parse()?;

        let route_segments = parse_route_segments(
            route.span(),
            children_routes.iter().flat_map(|f| f.named.iter()),
            &route.value(),
        )?
        .0;
        for seg in &route_segments {
            if let RouteSegment::CatchAll(name, _) = seg {
                return Err(syn::Error::new_spanned(
                    name,
                    format!(
                        "Catch-all segments are not allowed in nested routes: {}",
                        route.value()
                    ),
                ));
            }
        }

        Ok(Self {
            route: route.value(),
            segments: route_segments,
            index,
        })
    }
}

impl Nest {
    pub fn dynamic_segments(&self) -> impl Iterator<Item = TokenStream> + '_ {
        self.dynamic_segments_names().map(|i| quote! {#i})
    }

    pub fn dynamic_segments_names(&self) -> impl Iterator<Item = Ident> + '_ {
        self.segments.iter().filter_map(|seg| seg.name())
    }

    pub fn write(&self) -> TokenStream {
        let write_segments = self.segments.iter().map(|s| s.write_segment());

        quote! {
            {
                #(#write_segments)*
            }
        }
    }

    pub fn error_ident(&self) -> Ident {
        format_ident!("Nest{}ParseError", self.index)
    }

    pub fn error_variant(&self) -> Ident {
        format_ident!("Nest{}", self.index)
    }

    pub fn error_type(&self) -> TokenStream {
        let error_name = self.error_ident();

        let mut error_variants = Vec::new();
        let mut display_match = Vec::new();

        for (i, segment) in self.segments.iter().enumerate() {
            let error_name = segment.error_name(i);
            match segment {
                RouteSegment::Static(index) => {
                    error_variants.push(quote! { #error_name });
                    display_match.push(quote! { Self::#error_name => write!(f, "Static segment '{}' did not match", #index)? });
                }
                RouteSegment::Dynamic(ident, ty) => {
                    let missing_error = segment.missing_error_name().unwrap();
                    error_variants.push(quote! { #error_name(<#ty as dioxus_router::routable::FromRouteSegment>::Err) });
                    display_match.push(quote! { Self::#error_name(err) => write!(f, "Dynamic segment '({}:{})' did not match: {}", stringify!(#ident), stringify!(#ty), err)? });
                    error_variants.push(quote! { #missing_error });
                    display_match.push(quote! { Self::#missing_error => write!(f, "Dynamic segment '({}:{})' was missing", stringify!(#ident), stringify!(#ty))? });
                }
                _ => todo!(),
            }
        }

        quote! {
            #[allow(non_camel_case_types)]
            #[derive(Debug, PartialEq)]
            pub enum #error_name {
                #(#error_variants,)*
            }

            impl std::fmt::Display for #error_name {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    match self {
                        #(#display_match,)*
                    }
                    Ok(())
                }
            }
        }
    }
}
