use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::Ident;

use crate::nest::{Nest, NestId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LayoutId(pub usize);

#[derive(Debug)]
pub struct Layout {
    pub layout_name: Ident,
    pub comp: Ident,
    pub props_name: Ident,
    pub active_nests: Vec<NestId>,
}

impl Layout {
    pub fn routable_match(&self, nests: &[Nest]) -> TokenStream {
        let props_name = &self.props_name;
        let comp_name = &self.comp;
        let dynamic_segments = self
            .active_nests
            .iter()
            .flat_map(|id| nests[id.0].dynamic_segments());

        quote! {
            let comp = #props_name { #(#dynamic_segments,)* };
            let cx = cx.bump().alloc(Scoped {
                props: cx.bump().alloc(comp),
                scope: cx,
            });
            #comp_name(cx)
        }
    }
}

impl Layout {
    pub fn parse(input: syn::parse::ParseStream, active_nests: Vec<NestId>) -> syn::Result<Self> {
        // Then parse the layout name
        let _ = input.parse::<syn::Token![,]>();
        let layout_name: syn::Ident = input.parse()?;

        // Then parse the component name
        let _ = input.parse::<syn::Token![,]>();
        let comp: Ident = input.parse()?;

        // Then parse the props name
        let _ = input.parse::<syn::Token![,]>();
        let props_name: Ident = input
            .parse()
            .unwrap_or_else(|_| format_ident!("{}Props", comp.to_string()));

        Ok(Self {
            layout_name,
            comp,
            props_name,
            active_nests,
        })
    }
}
