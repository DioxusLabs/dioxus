use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::Path;

use crate::nest::{Nest, NestId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LayoutId(pub usize);

#[derive(Debug)]
pub struct Layout {
    pub comp: Path,
    pub props_name: Path,
    pub active_nests: Vec<NestId>,
}

impl Layout {
    pub fn routable_match(&self, nests: &[Nest]) -> TokenStream {
        let props_name = &self.props_name;
        let comp_name = &self.comp;
        let name_str = self.comp.segments.last().unwrap().ident.to_string();
        let dynamic_segments = self
            .active_nests
            .iter()
            .flat_map(|id| nests[id.0].dynamic_segments());

        quote! {
            let comp = #props_name { #(#dynamic_segments,)* };
            let dynamic = cx.component(#comp_name, comp, #name_str);
            render! {
                dynamic
            }
        }
    }
}

impl Layout {
    pub fn parse(input: syn::parse::ParseStream, active_nests: Vec<NestId>) -> syn::Result<Self> {
        // Then parse the component name
        let _ = input.parse::<syn::Token![,]>();
        let comp: Path = input.parse()?;

        // Then parse the props name
        let _ = input.parse::<syn::Token![,]>();
        let props_name = input.parse::<Path>().unwrap_or_else(|_| {
            let last = format_ident!("{}Props", comp.segments.last().unwrap().ident.to_string());
            let mut segments = comp.segments.clone();
            segments.pop();
            segments.push(last.into());
            Path {
                leading_colon: None,
                segments,
            }
        });

        Ok(Self {
            comp,
            props_name,
            active_nests,
        })
    }
}
