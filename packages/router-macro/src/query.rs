use quote::quote;
use syn::{Ident, Type};

use proc_macro2::TokenStream as TokenStream2;

#[derive(Debug)]
pub struct QuerySegment {
    pub ident: Ident,
    pub ty: Type,
}

impl QuerySegment {
    pub fn parse(&self) -> TokenStream2 {
        let ident = &self.ident;
        let ty = &self.ty;
        quote! {
            let #ident = <#ty as dioxus_router_core::router::FromQuery>::from_query(query);
        }
    }

    pub fn write(&self) -> TokenStream2 {
        let ident = &self.ident;
        quote! {
            write!(f, "?{}", #ident)?;
        }
    }

    pub fn name(&self) -> Ident {
        self.ident.clone()
    }

    pub fn ty(&self) -> &Type {
        &self.ty
    }
}
