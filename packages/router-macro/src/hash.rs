use quote::quote;
use syn::{Ident, Type};

use proc_macro2::TokenStream as TokenStream2;

#[derive(Debug)]
pub struct HashFragment {
    pub ident: Ident,
    pub ty: Type,
}

impl HashFragment {
    pub fn contains_ident(&self, ident: &Ident) -> bool {
        self.ident == *ident
    }

    pub fn parse(&self) -> TokenStream2 {
        let ident = &self.ident;
        let ty = &self.ty;
        quote! {
            let #ident = <#ty as dioxus_router::routable::FromHashFragment>::from_hash_fragment(&*hash);
        }
    }

    pub fn write(&self) -> TokenStream2 {
        let ident = &self.ident;
        quote! {
            write!(f, "#{}", #ident)?;
        }
    }

    pub fn parse_from_str<'a>(
        route_span: proc_macro2::Span,
        mut fields: impl Iterator<Item = (&'a Ident, &'a Type)>,
        hash: &str,
    ) -> syn::Result<Self> {
        // check if the route has a hash string
        let Some(hash) = hash.strip_prefix(':') else {
            return Err(syn::Error::new(
                route_span,
                "Failed to parse `:`. Hash fragments must be in the format '#:<field>'",
            ));
        };

        let hash_ident = Ident::new(hash, proc_macro2::Span::call_site());
        let field = fields.find(|(name, _)| *name == &hash_ident);

        let ty = if let Some((_, ty)) = field {
            ty.clone()
        } else {
            return Err(syn::Error::new(
                route_span,
                format!("Could not find a field with the name '{}'", hash_ident),
            ));
        };

        Ok(Self {
            ident: hash_ident,
            ty,
        })
    }
}
