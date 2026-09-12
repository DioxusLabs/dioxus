use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::LitStr;

use crate::{
    hash::HashFragment,
    nest::NestId,
    query::QuerySegment,
    segment::{RouteSegment, parse_route_segments, site_const},
};

#[derive(Debug)]
pub(crate) struct Redirect {
    pub route: LitStr,
    pub nests: Vec<NestId>,
    pub segments: Vec<RouteSegment>,
    pub query: Option<QuerySegment>,
    pub hash: Option<HashFragment>,
    pub function: syn::ExprClosure,
    pub index: usize,
}

impl Redirect {
    pub fn site_ident(&self) -> Ident {
        format_ident!("__SITE_REDIRECT_{}", self.index)
    }

    pub fn site_def(&self, error_type: &Ident) -> TokenStream {
        site_const(
            &self.site_ident(),
            error_type,
            "Redirect",
            &format!("Redirect{}ParseError", self.index),
            &self.route.value(),
        )
    }

    pub fn parse_query(&self) -> TokenStream {
        match &self.query {
            Some(query) => query.parse(),
            None => quote! {},
        }
    }

    pub fn parse_hash(&self) -> TokenStream {
        match &self.hash {
            Some(hash) => hash.parse(),
            None => quote! {},
        }
    }

    pub fn parse(
        input: syn::parse::ParseStream,
        active_nests: Vec<NestId>,
        index: usize,
    ) -> syn::Result<Self> {
        let path = input.parse::<syn::LitStr>()?;

        let _ = input.parse::<syn::Token![,]>();
        let function = input.parse::<syn::ExprClosure>()?;

        let mut closure_arguments = Vec::new();
        for arg in function.inputs.iter() {
            match arg {
                syn::Pat::Type(pat) => match &*pat.pat {
                    syn::Pat::Ident(ident) => {
                        closure_arguments.push((ident.ident.clone(), (*pat.ty).clone()));
                    }
                    _ => {
                        return Err(syn::Error::new_spanned(
                            arg,
                            "Expected closure argument to be a typed pattern",
                        ));
                    }
                },
                _ => {
                    return Err(syn::Error::new_spanned(
                        arg,
                        "Expected closure argument to be a typed pattern",
                    ));
                }
            }
        }

        let (segments, query, hash) = parse_route_segments(
            path.span(),
            #[allow(clippy::map_identity)]
            closure_arguments.iter().map(|(name, ty)| (name, ty)),
            &path.value(),
        )?;

        Ok(Redirect {
            route: path,
            nests: active_nests,
            segments,
            query,
            hash,
            function,
            index,
        })
    }
}
