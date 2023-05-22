use quote::{format_ident, quote, ToTokens};
use syn::parse::Parse;
use syn::parse::ParseStream;
use syn::{Ident, LitStr};

use proc_macro2::TokenStream as TokenStream2;

use crate::layout::Layout;
use crate::layout::LayoutId;
use crate::nest::Nest;
use crate::nest::NestId;
use crate::query::QuerySegment;
use crate::segment::parse_route_segments;
use crate::segment::RouteSegment;

struct RouteArgs {
    route: LitStr,
    comp_name: Option<Ident>,
    props_name: Option<Ident>,
}

impl Parse for RouteArgs {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let route = input.parse::<LitStr>()?;

        Ok(RouteArgs {
            route,
            comp_name: {
                let _ = input.parse::<syn::Token![,]>();
                input.parse().ok()
            },
            props_name: {
                let _ = input.parse::<syn::Token![,]>();
                input.parse().ok()
            },
        })
    }
}

#[derive(Debug)]
pub struct Route {
    pub file_based: bool,
    pub route_name: Ident,
    pub comp_name: Ident,
    pub props_name: Ident,
    pub route: String,
    pub segments: Vec<RouteSegment>,
    pub query: Option<QuerySegment>,
    pub nests: Vec<NestId>,
    pub layouts: Vec<LayoutId>,
    pub variant: syn::Variant,
    fields: syn::FieldsNamed,
}

impl Route {
    pub fn parse(
        nests: Vec<NestId>,
        layouts: Vec<LayoutId>,
        variant: syn::Variant,
    ) -> syn::Result<Self> {
        let route_attr = variant
            .attrs
            .iter()
            .find(|attr| attr.path.is_ident("route"))
            .ok_or_else(|| {
                syn::Error::new_spanned(
                    variant.clone(),
                    "Routable variants must have a #[route(...)] attribute",
                )
            })?;

        let route_name = variant.ident.clone();
        let args = route_attr.parse_args::<RouteArgs>()?;
        let route = args.route.value();
        let file_based = args.comp_name.is_none();
        let comp_name = args
            .comp_name
            .unwrap_or_else(|| format_ident!("{}", route_name));
        let props_name = args
            .props_name
            .unwrap_or_else(|| format_ident!("{}Props", comp_name));

        let named_fields = match &variant.fields {
            syn::Fields::Named(fields) => fields,
            _ => {
                return Err(syn::Error::new_spanned(
                    variant.clone(),
                    "Routable variants must have named fields",
                ))
            }
        };

        let (route_segments, query) =
            parse_route_segments(variant.ident.span(), named_fields.named.iter(), &route)?;

        Ok(Self {
            comp_name,
            props_name,
            route_name,
            segments: route_segments,
            route,
            file_based,
            query,
            nests,
            layouts,
            fields: named_fields.clone(),
            variant,
        })
    }

    pub fn display_match(&self, nests: &[Nest]) -> TokenStream2 {
        let name = &self.route_name;
        let dynamic_segments = self.dynamic_segments();
        let write_layouts = self.nests.iter().map(|id| nests[id.0].write());
        let write_segments = self.segments.iter().map(|s| s.write_segment());
        let write_query = self.query.as_ref().map(|q| q.write());

        quote! {
            Self::#name { #(#dynamic_segments,)* } => {
                #(#write_layouts)*
                #(#write_segments)*
                #write_query
            }
        }
    }

    pub fn routable_match(
        &self,
        layouts: &[Layout],
        nests: &[Nest],
        index: usize,
    ) -> Option<TokenStream2> {
        let name = &self.route_name;
        let dynamic_segments = self.dynamic_segments();

        match index.cmp(&self.layouts.len()) {
            std::cmp::Ordering::Less => {
                let layout = self.layouts[index];
                let render_layout = layouts[layout.0].routable_match(nests);
                // This is a layout
                Some(quote! {
                    #[allow(unused)]
                    Self::#name { #(#dynamic_segments,)* } => {
                        #render_layout
                    }
                })
            }
            std::cmp::Ordering::Equal => {
                let dynamic_segments_from_route = self.dynamic_segments();
                let props_name = &self.props_name;
                let comp_name = &self.comp_name;
                // This is the final route
                Some(quote! {
                    #[allow(unused)]
                    Self::#name { #(#dynamic_segments,)* } => {
                        let comp = #props_name { #(#dynamic_segments_from_route,)* };
                        let cx = cx.bump().alloc(Scoped {
                            props: cx.bump().alloc(comp),
                            scope: cx,
                        });
                        #comp_name(cx)
                    }
                })
            }
            _ => None,
        }
    }

    fn dynamic_segments(&self) -> impl Iterator<Item = TokenStream2> + '_ {
        self.fields.named.iter().map(|f| {
            let name = f.ident.as_ref().unwrap();
            quote! {#name}
        })
    }

    pub fn construct(&self, enum_name: Ident) -> TokenStream2 {
        let segments = self.dynamic_segments();
        let name = &self.route_name;

        quote! {
            #enum_name::#name {
                #(#segments,)*
            }
        }
    }

    pub fn error_ident(&self) -> Ident {
        format_ident!("{}ParseError", self.route_name)
    }

    pub fn error_type(&self) -> TokenStream2 {
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
                RouteSegment::CatchAll(ident, ty) => {
                    error_variants.push(quote! { #error_name(<#ty as dioxus_router_core::router::FromRouteSegments>::Err) });
                    display_match.push(quote! { Self::#error_name(err) => write!(f, "Catch-all segment '({}:{})' did not match: {}", stringify!(#ident), stringify!(#ty), err)? });
                }
            }
        }

        quote! {
            #[allow(non_camel_case_types)]
            #[derive(Debug, PartialEq)]
            pub enum #error_name {
                ExtraSegments(String),
                #(#error_variants,)*
            }

            impl std::fmt::Display for #error_name {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    match self {
                        Self::ExtraSegments(segments) => {
                            write!(f, "Found additional trailing segments: {segments}")?
                        }
                        #(#display_match,)*
                    }
                    Ok(())
                }
            }
        }
    }

    pub fn parse_query(&self) -> TokenStream2 {
        match &self.query {
            Some(query) => query.parse(),
            None => quote! {},
        }
    }

    pub fn variant(&self) -> TokenStream2 {
        let name = &self.route_name;
        let fields = self.fields.named.iter().map(|f| {
            let mut new = f.clone();
            new.attrs.retain(|a| {
                !a.path.is_ident("nest")
                    && !a.path.is_ident("end_nest")
                    && !a.path.is_ident("layout")
                    && !a.path.is_ident("end_layout")
            });
            new
        });

        quote! {
            #name { #(#fields,)* }
        }
    }
}

impl ToTokens for Route {
    fn to_tokens(&self, tokens: &mut quote::__private::TokenStream) {
        if !self.file_based {
            return;
        }

        let without_leading_slash = &self.route[1..];
        let route_path = std::path::Path::new(without_leading_slash);
        let with_extension = route_path.with_extension("rs");
        let dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        let dir = std::path::Path::new(&dir);
        let route = dir.join("src").join("pages").join(with_extension.clone());

        // check if the route exists or if not use the index route
        let route = if route.exists() && !without_leading_slash.is_empty() {
            with_extension.to_str().unwrap().to_string()
        } else {
            route_path.join("index.rs").to_str().unwrap().to_string()
        };

        let route_name: Ident = self.route_name.clone();
        let prop_name = &self.props_name;

        tokens.extend(quote!(
            #[path = #route]
            #[allow(non_snake_case)]
            mod #route_name;
            pub use #route_name::{#prop_name, #route_name};
        ));
    }
}
