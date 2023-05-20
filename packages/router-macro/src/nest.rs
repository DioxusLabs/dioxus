use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{parse::Parse, Ident, LitStr};

use crate::segment::{parse_route_segments, RouteSegment};

pub enum Nest {
    Static(String),
    Layout(Layout),
}

impl Parse for Nest {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        // First parse the route
        let route: LitStr = input.parse()?;
        let is_dynamic = route.value().contains('(');

        if !input.is_empty() || is_dynamic {
            // Then parse the layout name
            let _ = input.parse::<syn::Token![,]>();
            let layout_name: syn::Ident = input.parse()?;
            let layout_fields: syn::FieldsNamed = input.parse()?;

            // Then parse the component name
            let _ = input.parse::<syn::Token![,]>();
            let comp: Ident = input.parse()?;

            // Then parse the props name
            let _ = input.parse::<syn::Token![,]>();
            let props_name: Ident = input
                .parse()
                .unwrap_or_else(|_| format_ident!("{}Props", comp.to_string()));

            let route_segments =
                parse_route_segments(&layout_name, &layout_fields, &route.value())?.0;
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

            Ok(Self::Layout(Layout {
                route: route.value(),
                segments: route_segments,
                layout_name,
                comp,
                props_name,
                layout_fields,
            }))
        } else {
            Ok(Self::Static(route.value()))
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LayoutId(pub usize);

#[derive(Debug)]
pub struct Layout {
    pub route: String,
    pub segments: Vec<RouteSegment>,
    pub layout_name: Ident,
    pub layout_fields: syn::FieldsNamed,
    pub comp: Ident,
    pub props_name: Ident,
}

impl Layout {
    pub fn add_static_prefix(&mut self, prefix: &str) {
        self.route = format!("{}{}", prefix, self.route);
        self.segments.push(RouteSegment::Static(prefix.to_string()));
    }

    pub fn dynamic_segments(&self) -> impl Iterator<Item = TokenStream> + '_ {
        self.segments
            .iter()
            .filter_map(|seg| seg.name())
            .map(|i| quote! {#i})
    }

    pub fn dynamic_segment_types(&self) -> impl Iterator<Item = TokenStream> + '_ {
        self.segments
            .iter()
            .filter_map(|seg| seg.ty())
            .map(|ty| quote! {#ty})
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
        format_ident!("{}LayoutParseError", self.layout_name)
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
                    error_variants.push(quote! { #error_name(<#ty as dioxus_router_core::router::FromRouteSegment>::Err) });
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

    pub fn routable_match(&self) -> TokenStream {
        let props_name = &self.props_name;
        let comp_name = &self.comp;
        let dynamic_segments_from_route = self
            .segments
            .iter()
            .filter_map(|seg| seg.name())
            .map(|seg| quote! { #seg });

        quote! {
            let comp = #props_name { #(#dynamic_segments_from_route,)* };
            let cx = cx.bump().alloc(Scoped {
                props: cx.bump().alloc(comp),
                scope: cx,
            });
            #comp_name(cx)
        }
    }
}
