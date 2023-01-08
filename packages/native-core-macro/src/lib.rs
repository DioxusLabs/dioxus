extern crate proc_macro;

mod sorted_slice;

use proc_macro::TokenStream;
use quote::quote;
use sorted_slice::StrSlice;
use syn::{self, parse_macro_input};

/// Sorts a slice of string literals at compile time.
#[proc_macro]
pub fn sorted_str_slice(input: TokenStream) -> TokenStream {
    let slice: StrSlice = parse_macro_input!(input as StrSlice);
    let strings = slice.map.values();
    quote!([#(#strings, )*]).into()
}

/// Derive's the state from any members that implement the Pass trait
#[proc_macro_derive(State, attributes(skip, skip_clone))]
pub fn state_macro_derive(input: TokenStream) -> TokenStream {
    let ast = syn::parse(input).unwrap();
    impl_derive_macro(&ast)
}

fn impl_derive_macro(ast: &syn::DeriveInput) -> TokenStream {
    let custom_type = ast
        .attrs
        .iter()
        .find(|a| a.path.is_ident("state"))
        .and_then(|attr| {
            // parse custom_type = "MyType"
            let assignment = attr.parse_args::<syn::Expr>().unwrap();
            if let syn::Expr::Assign(assign) = assignment {
                let (left, right) = (&*assign.left, &*assign.right);
                if let syn::Expr::Path(e) = left {
                    let path = &e.path;
                    if let Some(ident) = path.get_ident() {
                        if ident == "custom_value" {
                            return match right {
                                syn::Expr::Path(e) => {
                                    let path = &e.path;
                                    Some(quote! {#path})
                                }
                                _ => None,
                            };
                        }
                    }
                }
            }
            None
        })
        .unwrap_or(quote! {()});
    let type_name = &ast.ident;
    let fields: Vec<_> = match &ast.data {
        syn::Data::Struct(data) => match &data.fields {
            syn::Fields::Named(e) => &e.named,
            syn::Fields::Unnamed(_) => todo!("unnamed fields"),
            syn::Fields::Unit => todo!("unit structs"),
        }
        .iter()
        .collect(),
        _ => unimplemented!(),
    };

    let types = fields
        .iter()
        .filter(|field| !field.attrs.iter().any(|attr| attr.path.is_ident("skip")))
        .map(|field| &field.ty);

    let gen = quote! {
        impl dioxus_native_core::State<#custom_type> for #type_name {
            fn create_passes() -> Box<[dioxus_native_core::TypeErasedPass<Self>]> {
                Box::new([
                    #(
                        <#types as dioxus_native_core::Pass>::to_type_erased()
                    ),*
                ])
            }
        }
    };

    gen.into()
}

/// Derive's the state from any elements that have a node_dep_state, child_dep_state, parent_dep_state, or state attribute.
#[proc_macro_derive(AnyMapLike)]
pub fn anymap_like_macro_derive(input: TokenStream) -> TokenStream {
    let ast = syn::parse(input).unwrap();
    impl_anymap_like_derive_macro(&ast)
}

fn impl_anymap_like_derive_macro(ast: &syn::DeriveInput) -> TokenStream {
    let type_name = &ast.ident;
    let fields: Vec<_> = match &ast.data {
        syn::Data::Struct(data) => match &data.fields {
            syn::Fields::Named(e) => &e.named,
            syn::Fields::Unnamed(_) => todo!("unnamed fields"),
            syn::Fields::Unit => todo!("unit structs"),
        }
        .iter()
        .collect(),
        _ => unimplemented!(),
    };

    let names: Vec<_> = fields
        .iter()
        .map(|field| field.ident.as_ref().unwrap())
        .collect();
    let types: Vec<_> = fields.iter().map(|field| &field.ty).collect();

    let gen = quote! {
        impl dioxus_native_core::AnyMapLike for #type_name {
            fn get<T: std::any::Any>(&self) -> Option<&T> {
                #(
                    if std::any::TypeId::of::<T>() == std::any::TypeId::of::<#types>() {
                        return unsafe { Some(&*(&self.#names as *const _ as *const T)) }
                    }
                )*
                None
            }

            fn get_mut<T: std::any::Any>(&mut self) -> Option<&mut T> {
                #(
                    if std::any::TypeId::of::<T>() == std::any::TypeId::of::<#types>() {
                        return unsafe { Some(&mut *(&mut self.#names as *mut _ as *mut T)) }
                    }
                )*
                None
            }
        }
    };

    gen.into()
}
