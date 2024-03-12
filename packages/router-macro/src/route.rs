use quote::{format_ident, quote};
use syn::parse::Parse;
use syn::parse::ParseStream;
use syn::parse_quote;
use syn::Field;
use syn::Path;
use syn::Type;
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
        })
    }
}

struct ChildArgs {
    route: LitStr,
}

impl Parse for ChildArgs {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let route = input.parse::<LitStr>()?;

        Ok(ChildArgs { route })
    }
}

#[derive(Debug)]
pub(crate) struct Route {
    pub route_name: Ident,
    pub ty: RouteType,
    pub route: String,
    pub segments: Vec<RouteSegment>,
    pub query: Option<QuerySegment>,
    pub nests: Vec<NestId>,
    pub layouts: Vec<LayoutId>,
    fields: Vec<(Ident, Type)>,
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
            .find(|attr| attr.path().is_ident("route"));
        let route;
        let ty;
        let route_name = variant.ident.clone();
        match route_attr {
            Some(attr) => {
                let args = attr.parse_args::<RouteArgs>()?;
                let comp_name = args.comp_name.unwrap_or_else(|| parse_quote!(#route_name));
                ty = RouteType::Leaf {
                    component: comp_name,
                };
                route = args.route.value();
            }
            None => {
                if let Some(route_attr) = variant
                    .attrs
                    .iter()
                    .find(|attr| attr.path().is_ident("child"))
                {
                    let args = route_attr.parse_args::<ChildArgs>()?;
                    route = args.route.value();
                    match &variant.fields {
                        syn::Fields::Named(fields) => {
                            // find either a field with #[child] or a field named "child"
                            let child_field = fields.named.iter().find(|f| {
                                f.attrs
                                    .iter()
                                    .any(|attr| attr.path().is_ident("child"))
                                    || *f.ident.as_ref().unwrap() == "child"
                            });
                            match child_field{
                                Some(child) => {
                                    ty = RouteType::Child(child.clone());
                                }
                                None => {
                                    return Err(syn::Error::new_spanned(
                                        variant.clone(),
                                        "Routable variants with a #[child(..)] attribute must have a field named \"child\" or a field with a #[child] attribute",
                                    ));
                                }
                            }
                        }
                        _ => {
                            return Err(syn::Error::new_spanned(
                                variant.clone(),
                                "Routable variants with a #[child(..)] attribute must have named fields",
                            ))
                        }
                    }
                } else {
                    return Err(syn::Error::new_spanned(
                            variant.clone(),
                            "Routable variants must either have a #[route(..)] attribute or a #[child(..)] attribute",
                        ));
                }
            }
        };

        let fields = match &variant.fields {
            syn::Fields::Named(fields) => fields
                .named
                .iter()
                .filter_map(|f| {
                    if let RouteType::Child(child) = &ty {
                        if f.ident == child.ident {
                            return None;
                        }
                    }
                    Some((f.ident.clone().unwrap(), f.ty.clone()))
                })
                .collect(),
            _ => Vec::new(),
        };

        let (route_segments, query) = {
            parse_route_segments(
                variant.ident.span(),
                fields.iter().map(|f| (&f.0, &f.1)),
                &route,
            )?
        };

        Ok(Self {
            ty,
            route_name,
            segments: route_segments,
            route,
            query,
            nests,
            layouts,
            fields,
        })
    }

    pub fn display_match(&self, nests: &[Nest]) -> TokenStream2 {
        let name = &self.route_name;
        let dynamic_segments = self.dynamic_segments();
        let write_query = self.query.as_ref().map(|q| q.write());

        match &self.ty {
            RouteType::Child(field) => {
                let write_nests = self.nests.iter().map(|id| nests[id.0].write());
                let write_segments = self.segments.iter().map(|s| s.write_segment());
                let child = field.ident.as_ref().unwrap();
                quote! {
                    Self::#name { #(#dynamic_segments,)* #child } => {
                        use std::fmt::Display;
                        use std::fmt::Write;
                        let mut route = String::new();
                        {
                            let f = &mut route;
                            #(#write_nests)*
                            #(#write_segments)*
                        }
                        if route.ends_with('/') {
                            route.pop();
                        }
                        f.write_str(&route)?;
                        #child.fmt(f)?;
                    }
                }
            }
            RouteType::Leaf { .. } => {
                let write_nests = self.nests.iter().map(|id| nests[id.0].write());
                let write_segments = self.segments.iter().map(|s| s.write_segment());
                quote! {
                    Self::#name { #(#dynamic_segments,)* } => {
                        #(#write_nests)*
                        #(#write_segments)*
                        #write_query
                    }
                }
            }
        }
    }

    pub fn routable_match(&self, layouts: &[Layout], nests: &[Nest]) -> TokenStream2 {
        let name = &self.route_name;

        let mut tokens = TokenStream2::new();

        // First match all layouts
        for (idx, layout_id) in self.layouts.iter().copied().enumerate() {
            let render_layout = layouts[layout_id.0].routable_match(nests);
            let dynamic_segments = self.dynamic_segments();
            let mut field_name = None;
            if let RouteType::Child(field) = &self.ty {
                field_name = field.ident.as_ref();
            }
            let field_name = field_name.map(|f| quote!(#f,));
            // This is a layout
            tokens.extend(quote! {
                #[allow(unused)]
                (#idx, Self::#name { #(#dynamic_segments,)* #field_name .. }) => {
                    #render_layout
                }
            });
        }

        // Then match the route
        let last_index = self.layouts.len();
        tokens.extend(match &self.ty {
            RouteType::Child(field) => {
                let field_name = field.ident.as_ref().unwrap();
                quote! {
                    #[allow(unused)]
                    (#last_index.., Self::#name { #field_name, .. }) => {
                        #field_name.render(level - #last_index)
                    }
                }
            }
            RouteType::Leaf { component } => {
                let dynamic_segments = self.dynamic_segments();
                let dynamic_segments_from_route = self.dynamic_segments();
                quote! {
                    #[allow(unused)]
                    (#last_index, Self::#name { #(#dynamic_segments,)* }) => {
                        rsx! {
                            #component {
                                #(#dynamic_segments_from_route: #dynamic_segments_from_route,)*
                            }
                        }
                    }
                }
            }
        });

        tokens
    }

    fn dynamic_segments(&self) -> impl Iterator<Item = TokenStream2> + '_ {
        self.fields.iter().map(|(name, _)| {
            quote! {#name}
        })
    }

    pub fn construct(&self, nests: &[Nest], enum_name: Ident) -> TokenStream2 {
        let segments = self.fields.iter().map(|(name, _)| {
            let mut from_route = false;

            for id in &self.nests {
                let nest = &nests[id.0];
                if nest.dynamic_segments_names().any(|i| &i == name) {
                    from_route = true
                }
            }
            for segment in &self.segments {
                if segment.name().as_ref() == Some(name) {
                    from_route = true
                }
            }
            if let Some(query) = &self.query {
                if query.contains_ident(name) {
                    from_route = true
                }
            }

            if from_route {
                quote! {#name}
            } else {
                quote! {#name: Default::default()}
            }
        });
        match &self.ty {
            RouteType::Child(field) => {
                let name = &self.route_name;
                let child_name = field.ident.as_ref().unwrap();

                quote! {
                    #enum_name::#name {
                        #child_name,
                        #(#segments,)*
                    }
                }
            }
            RouteType::Leaf { .. } => {
                let name = &self.route_name;

                quote! {
                    #enum_name::#name {
                        #(#segments,)*
                    }
                }
            }
        }
    }

    pub fn error_ident(&self) -> Ident {
        format_ident!("{}ParseError", self.route_name)
    }

    pub fn error_type(&self) -> TokenStream2 {
        let error_name = self.error_ident();
        let child_type = match &self.ty {
            RouteType::Child(field) => Some(&field.ty),
            RouteType::Leaf { .. } => None,
        };

        create_error_type(error_name, &self.segments, child_type)
    }

    pub fn parse_query(&self) -> TokenStream2 {
        match &self.query {
            Some(query) => query.parse(),
            None => quote! {},
        }
    }
}

#[derive(Debug)]
pub(crate) enum RouteType {
    Child(Field),
    Leaf { component: Path },
}
