use proc_macro2::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{Data, DeriveInput, Fields, Ident, LitStr, Variant};

const AT_ATTR_IDENT: &str = "at";
const NOT_FOUND_ATTR_IDENT: &str = "not_found";

pub struct Routable {
    ident: Ident,
    ats: Vec<LitStr>,
    variants: Punctuated<Variant, syn::token::Comma>,
    not_found_route: Option<Ident>,
}

impl Parse for Routable {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let DeriveInput { ident, data, .. } = input.parse()?;

        let data = match data {
            Data::Enum(data) => data,
            Data::Struct(s) => {
                return Err(syn::Error::new(
                    s.struct_token.span(),
                    "expected enum, found struct",
                ))
            }
            Data::Union(u) => {
                return Err(syn::Error::new(
                    u.union_token.span(),
                    "expected enum, found union",
                ))
            }
        };

        let (not_found_route, ats) = parse_variants_attributes(&data.variants)?;

        Ok(Self {
            ident,
            variants: data.variants,
            ats,
            not_found_route,
        })
    }
}

fn parse_variants_attributes(
    variants: &Punctuated<Variant, syn::token::Comma>,
) -> syn::Result<(Option<Ident>, Vec<LitStr>)> {
    let mut not_founds = vec![];
    let mut ats: Vec<LitStr> = vec![];

    let mut not_found_attrs = vec![];

    for variant in variants.iter() {
        if let Fields::Unnamed(ref field) = variant.fields {
            return Err(syn::Error::new(
                field.span(),
                "only named fields are supported",
            ));
        }

        let attrs = &variant.attrs;
        let at_attrs = attrs
            .iter()
            .filter(|attr| attr.path.is_ident(AT_ATTR_IDENT))
            .collect::<Vec<_>>();

        let attr = match at_attrs.len() {
            1 => *at_attrs.first().unwrap(),
            0 => {
                return Err(syn::Error::new(
                    variant.span(),
                    format!(
                        "{} attribute must be present on every variant",
                        AT_ATTR_IDENT
                    ),
                ))
            }
            _ => {
                return Err(syn::Error::new_spanned(
                    quote! { #(#at_attrs)* },
                    format!("only one {} attribute must be present", AT_ATTR_IDENT),
                ))
            }
        };

        let lit = attr.parse_args::<LitStr>()?;
        ats.push(lit);

        for attr in attrs.iter() {
            if attr.path.is_ident(NOT_FOUND_ATTR_IDENT) {
                not_found_attrs.push(attr);
                not_founds.push(variant.ident.clone())
            }
        }
    }

    if not_founds.len() > 1 {
        return Err(syn::Error::new_spanned(
            quote! { #(#not_found_attrs)* },
            format!("there can only be one {}", NOT_FOUND_ATTR_IDENT),
        ));
    }

    Ok((not_founds.into_iter().next(), ats))
}

impl Routable {
    fn build_from_path(&self) -> TokenStream {
        let from_path_matches = self.variants.iter().enumerate().map(|(i, variant)| {
            let ident = &variant.ident;
            let right = match &variant.fields {
                Fields::Unit => quote! { Self::#ident },
                Fields::Named(field) => {
                    let fields = field.named.iter().map(|it| {
                        //named fields have idents
                        it.ident.as_ref().unwrap()
                    });
                    quote! { Self::#ident { #(#fields: params.get(stringify!(#fields))?.parse().ok()?,)* } }
                }
                Fields::Unnamed(_) => unreachable!(), // already checked
            };

            let left = self.ats.get(i).unwrap();
            quote! {
                #left => ::std::option::Option::Some(#right)
            }
        });

        quote! {
            fn from_path(path: &str, params: &::std::collections::HashMap<&str, &str>) -> ::std::option::Option<Self> {
                match path {
                    #(#from_path_matches),*,
                    _ => ::std::option::Option::None,
                }
            }
        }
    }

    fn build_to_path(&self) -> TokenStream {
        let to_path_matches = self.variants.iter().enumerate().map(|(i, variant)| {
            let ident = &variant.ident;
            let mut right = self.ats.get(i).unwrap().value();

            match &variant.fields {
                Fields::Unit => quote! { Self::#ident => ::std::string::ToString::to_string(#right) },
                Fields::Named(field) => {
                    let fields = field
                        .named
                        .iter()
                        .map(|it| it.ident.as_ref().unwrap())
                        .collect::<Vec<_>>();

                    for field in fields.iter() {
                        // :param -> {param}
                        // so we can pass it to `format!("...", param)`
                        right = right.replace(&format!(":{}", field), &format!("{{{}}}", field))
                    }

                    quote! {
                        Self::#ident { #(#fields),* } => ::std::format!(#right, #(#fields = #fields),*)
                    }
                }
                Fields::Unnamed(_) => unreachable!(), // already checked
            }
        });

        quote! {
            fn to_path(&self) -> ::std::string::String {
                match self {
                    #(#to_path_matches),*,
                }
            }
        }
    }
}

pub fn routable_derive_impl(input: Routable) -> TokenStream {
    let Routable {
        ats,
        not_found_route,
        ident,
        ..
    } = &input;

    let from_path = input.build_from_path();
    let to_path = input.build_to_path();

    let not_found_route = match not_found_route {
        Some(route) => quote! { ::std::option::Option::Some(Self::#route) },
        None => quote! { ::std::option::Option::None },
    };

    let cache_thread_local_ident = Ident::new(
        &format!("__{}_ROUTER_CURRENT_ROUTE_CACHE", ident),
        ident.span(),
    );

    quote! {
        // ::std::thread_local! {
        //     #[doc(hidden)]
        //     #[allow(non_upper_case_globals)]
        //     static #cache_thread_local_ident: ::std::cell::RefCell<::std::option::Option<#ident>> = ::std::cell::RefCell::new(::std::option::Option::None);
        // }

        // #[automatically_derived]
        // impl ::dioxus::router::Routable for #ident {
        //     #from_path
        //     #to_path

        //     // fn routes() -> ::std::vec::Vec<&'static str> {
        //     //     ::std::vec![#(#ats),*]
        //     // }

        //     fn not_found_route() -> ::std::option::Option<Self> {
        //         #not_found_route
        //     }

        //     // fn current_route() -> ::std::option::Option<Self> {
        //     //     #cache_thread_local_ident.with(|val| ::std::clone::Clone::clone(&*val.borrow()))
        //     // }

        //     fn recognize(pathname: &str) -> ::std::option::Option<Self> {
        //         todo!()
        //         // ::std::thread_local! {
        //         //     static ROUTER: ::dioxus::router::__macro::Router = ::dioxus::router::__macro::build_router::<#ident>();
        //         // }
        //         // let route = ROUTER.with(|router| ::dioxus::router::__macro::recognize_with_router(router, pathname));
        //         // {
        //         //     let route = ::std::clone::Clone::clone(&route);
        //         //     #cache_thread_local_ident.with(move |val| {
        //         //         *val.borrow_mut() = route;
        //         //     });
        //         // }
        //         // route
        //     }

        //     // fn cleanup() {
        //     //     #cache_thread_local_ident.with(move |val| {
        //     //         *val.borrow_mut() = ::std::option::Option::None;
        //     //     });
        //     // }
        // }
    }
}
