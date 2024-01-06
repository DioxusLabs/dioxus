use quote::quote;
use syn::{Ident, Type};

use proc_macro2::TokenStream as TokenStream2;

#[derive(Debug)]
pub enum QuerySegment {
    Single(FullQuerySegment),
    Segments(Vec<QueryArgument>),
}

impl QuerySegment {
    pub fn contains_ident(&self, ident: &Ident) -> bool {
        match self {
            QuerySegment::Single(segment) => segment.ident == *ident,
            QuerySegment::Segments(segments) => {
                segments.iter().any(|segment| segment.ident == *ident)
            }
        }
    }

    pub fn parse(&self) -> TokenStream2 {
        match self {
            QuerySegment::Single(segment) => segment.parse(),
            QuerySegment::Segments(segments) => {
                let mut tokens = TokenStream2::new();
                tokens.extend(quote! { let split_query: std::collections::HashMap<&str, &str> = query.split('&').filter_map(|s| s.split_once('=')).collect(); });
                for segment in segments {
                    tokens.extend(segment.parse());
                }
                tokens
            }
        }
    }

    pub fn write(&self) -> TokenStream2 {
        match self {
            QuerySegment::Single(segment) => segment.write(),
            QuerySegment::Segments(segments) => {
                let mut tokens = TokenStream2::new();
                tokens.extend(quote! { write!(f, "?")?; });
                let mut segments_iter = segments.iter();
                if let Some(first_segment) = segments_iter.next() {
                    tokens.extend(first_segment.write());
                }
                for segment in segments_iter {
                    tokens.extend(quote! { write!(f, "&")?; });
                    tokens.extend(segment.write());
                }
                tokens
            }
        }
    }
}

#[derive(Debug)]
pub struct FullQuerySegment {
    pub ident: Ident,
    pub ty: Type,
}

impl FullQuerySegment {
    pub fn parse(&self) -> TokenStream2 {
        let ident = &self.ident;
        let ty = &self.ty;
        quote! {
            let #ident = <#ty as dioxus_router::routable::FromQuery>::from_query(&*query);
        }
    }

    pub fn write(&self) -> TokenStream2 {
        let ident = &self.ident;
        quote! {
            write!(f, "?{}", #ident)?;
        }
    }
}

#[derive(Debug)]
pub struct QueryArgument {
    pub ident: Ident,
    pub ty: Type,
}

impl QueryArgument {
    pub fn parse(&self) -> TokenStream2 {
        let ident = &self.ident;
        let ty = &self.ty;
        quote! {
            let #ident = match split_query.get(stringify!(#ident)) {
                Some(query_argument) => <#ty as dioxus_router::routable::FromQueryArgument>::from_query_argument(query_argument).unwrap_or_default(),
                None => <#ty as Default>::default(),
            };
        }
    }

    pub fn write(&self) -> TokenStream2 {
        let ident = &self.ident;
        quote! {
            write!(f, "{}={}", stringify!(#ident), #ident)?;
        }
    }
}
