use proc_macro2::TokenStream;
use quote::quote;
use syn::Path;

use crate::nest::{Nest, NestId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LayoutId(pub usize);

#[derive(Debug)]
pub struct Layout {
    pub comp: Path,
    pub active_nests: Vec<NestId>,
}

impl Layout {
    pub fn routable_match(&self, nests: &[Nest]) -> TokenStream {
        let comp_name = &self.comp;
        let dynamic_segments = self
            .active_nests
            .iter()
            .flat_map(|id| nests[id.0].dynamic_segments());

        quote! {
            rsx! {
                #comp_name { #(#dynamic_segments: #dynamic_segments,)* }
            }
        }
    }
}

impl Layout {
    pub fn parse(input: syn::parse::ParseStream, active_nests: Vec<NestId>) -> syn::Result<Self> {
        // Then parse the component name
        let _ = input.parse::<syn::Token![,]>();
        let comp: Path = input.parse()?;

        Ok(Self { comp, active_nests })
    }
}
