use quote::{format_ident, quote};
use syn::parse::Parse;
use syn::parse::ParseStream;
use syn::parse_quote;
use syn::Path;
use syn::{Ident, LitStr};

use proc_macro2::TokenStream as TokenStream2;

use crate::layout::Layout;
use crate::layout::LayoutId;
use crate::nest::Nest;
use crate::nest::NestId;
use crate::query::QuerySegment;
use crate::segment::create_error_type;
use crate::segment::parse_route_segments;
use crate::segment::RouteSegment;

struct RouteArgs {
    route: LitStr,
    comp_name: Option<Path>,
    props_name: Option<Path>,
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
    pub route_name: Ident,
    pub comp_name: Path,
    pub props_name: Path,
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
        let comp_name = args.comp_name.unwrap_or_else(|| parse_quote!(#route_name));
        let props_name = args.props_name.unwrap_or_else(|| {
            let last = format_ident!(
                "{}Props",
                comp_name.segments.last().unwrap().ident.to_string()
            );
            let mut segments = comp_name.segments.clone();
            segments.pop();
            segments.push(last.into());
            Path {
                leading_colon: None,
                segments,
            }
        });

        let named_fields = match &variant.fields {
            syn::Fields::Named(fields) => fields,
            _ => {
                return Err(syn::Error::new_spanned(
                    variant.clone(),
                    "Routable variants must have named fields",
                ))
            }
        };

        let (route_segments, query) = parse_route_segments(
            variant.ident.span(),
            named_fields
                .named
                .iter()
                .map(|f| (f.ident.as_ref().unwrap(), &f.ty)),
            &route,
        )?;

        Ok(Self {
            comp_name,
            props_name,
            route_name,
            segments: route_segments,
            route,
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

    pub fn construct(&self, nests: &[Nest], enum_name: Ident) -> TokenStream2 {
        let segments = self.fields.named.iter().map(|f| {
            let mut from_route = false;
            for id in &self.nests {
                let nest = &nests[id.0];
                if nest
                    .dynamic_segments_names()
                    .any(|i| &i == f.ident.as_ref().unwrap())
                {
                    from_route = true
                }
            }
            for segment in &self.segments {
                if let RouteSegment::Dynamic(name, ..) = segment {
                    if name == f.ident.as_ref().unwrap() {
                        from_route = true
                    }
                }
            }

            let name = f.ident.as_ref().unwrap();

            if from_route {
                quote! {#name}
            } else {
                quote! {#name: Default::default()}
            }
        });
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

        create_error_type(error_name, &self.segments)
    }

    pub fn parse_query(&self) -> TokenStream2 {
        match &self.query {
            Some(query) => query.parse(),
            None => quote! {},
        }
    }
}
