use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{ExprClosure, Ident, LitStr, Type, Variant};

use crate::{
    hash::HashFragment,
    nest::NestId,
    query::QuerySegment,
    segment::{create_error_type, parse_route_segments, RouteSegment},
};

#[derive(Debug)]
pub(crate) struct Redirect {
    pub route: LitStr,
    pub nests: Vec<NestId>,
    pub segments: Vec<RouteSegment>,
    pub query: Option<QuerySegment>,
    pub hash: Option<HashFragment>,
    pub function: RedirectExpr,
    pub index: usize,
}

#[derive(Debug)]
pub(crate) enum RedirectExpr {
    TypedClosure(ExprClosure),
    RouteVariant(Variant),
}

impl Redirect {
    pub fn error_ident(&self) -> Ident {
        format_ident!("Redirect{}ParseError", self.index)
    }

    pub fn error_variant(&self) -> Ident {
        format_ident!("Redirect{}", self.index)
    }

    pub fn error_type(&self) -> TokenStream {
        let error_name = self.error_ident();

        create_error_type(error_name, &self.segments, None)
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

        let function = match input.parse::<syn::ExprClosure>() {
            Ok(c) => RedirectExpr::TypedClosure(c),
            Err(_) => RedirectExpr::RouteVariant(
                input
                    .parse::<syn::Variant>()
                    .expect("redirect function must be a closure or a route variant"),
            ),
        };

        let (segments, query, hash) = match function {
            RedirectExpr::TypedClosure(ref fun) => Self::parse_closure_redirect(&path, fun)?,
            RedirectExpr::RouteVariant(ref fun) => Self::parse_field_redirect(&path, fun)?,
        };


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

    fn parse_closure_redirect(
        path: &LitStr,
        function: &ExprClosure,
    ) -> syn::Result<(
        Vec<RouteSegment>,
        Option<QuerySegment>,
        Option<HashFragment>,
    )> {
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
                        ))
                    }
                },
                _ => {
                    return Err(syn::Error::new_spanned(
                        arg,
                        "Expected closure argument to be a typed pattern",
                    ))
                }
            }
        }

        Ok(parse_route_segments(
            path.span(),
            #[allow(clippy::map_identity)]
            closure_arguments.iter().map(|(name, ty)| (name, ty)),
            &path.value(),
        )?)
    }

    fn parse_field_redirect(
        path: &LitStr,
        _variant: &Variant,
    ) -> syn::Result<(
        Vec<RouteSegment>,
        Option<QuerySegment>,
        Option<HashFragment>,
    )> {
        Ok(parse_route_segments(
            path.span(),
            std::iter::empty::<(&Ident, &Type)>(),
            &path.value(),
        )?)
    }
}
