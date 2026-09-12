use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Ident, LitStr};

use crate::segment::{RouteSegment, parse_route_segments, site_const};

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
            children_routes
                .iter()
                .flat_map(|f| f.named.iter())
                .map(|f| (f.ident.as_ref().unwrap(), &f.ty)),
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

    pub fn site_ident(&self) -> Ident {
        format_ident!("__SITE_NEST_{}", self.index)
    }

    pub fn site_def(&self, error_type: &Ident) -> TokenStream {
        site_const(
            &self.site_ident(),
            error_type,
            "Nest",
            &format!("Nest{}ParseError", self.index),
            &self.route,
        )
    }
}
