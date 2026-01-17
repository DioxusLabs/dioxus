use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{parse_macro_input, DeriveInput, LitInt, Path};
use syn::{parse_quote, Generics, WhereClause, WherePredicate};

fn add_bounds(where_clause: &mut Option<WhereClause>, generics: &Generics, krate: &Path) {
    let bounds = generics.params.iter().filter_map(|param| match param {
        syn::GenericParam::Type(ty) => {
            Some::<WherePredicate>(parse_quote! { #ty: #krate::SerializeConst, })
        }
        syn::GenericParam::Lifetime(_) => None,
        syn::GenericParam::Const(_) => None,
    });
    if let Some(clause) = where_clause {
        clause.predicates.extend(bounds);
    } else {
        *where_clause = Some(parse_quote! { where #(#bounds)* });
    }
}

/// Derive the const serialize trait for a struct
#[proc_macro_derive(SerializeConst, attributes(const_serialize))]
pub fn derive_parse(raw_input: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let input = parse_macro_input!(raw_input as DeriveInput);
    let krate = input.attrs.iter().find_map(|attr| {
        attr.path()
            .is_ident("const_serialize")
            .then(|| {
                let mut path = None;
                if let Err(err) = attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("crate") {
                        let ident: Path = meta.value()?.parse()?;
                        path = Some(ident);
                    }
                    Ok(())
                }) {
                    return Some(Err(err));
                }
                path.map(Ok)
            })
            .flatten()
    });
    let krate = match krate {
        Some(Ok(path)) => path,
        Some(Err(err)) => return err.into_compile_error().into(),
        None => parse_quote! { const_serialize },
    };

    match input.data {
        syn::Data::Struct(data) => match data.fields {
            syn::Fields::Unnamed(_) | syn::Fields::Named(_) => {
                let ty = &input.ident;
                let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
                let mut where_clause = where_clause.cloned();
                add_bounds(&mut where_clause, &input.generics, &krate);
                let field_names = data.fields.iter().enumerate().map(|(i, field)| {
                    field
                        .ident
                        .as_ref()
                        .map(|ident| ident.to_token_stream())
                        .unwrap_or_else(|| {
                            LitInt::new(&i.to_string(), proc_macro2::Span::call_site())
                                .into_token_stream()
                        })
                });
                let field_types = data.fields.iter().map(|field| &field.ty);
                quote! {
                    unsafe impl #impl_generics #krate::SerializeConst for #ty #ty_generics #where_clause {
                        const MEMORY_LAYOUT: #krate::Layout = #krate::Layout::Struct(#krate::StructLayout::new(
                            std::mem::size_of::<Self>(),
                            &[#(
                                #krate::StructFieldLayout::new(
                                    stringify!(#field_names),
                                    std::mem::offset_of!(#ty, #field_names),
                                    <#field_types as #krate::SerializeConst>::MEMORY_LAYOUT,
                                ),
                            )*],
                        ));
                    }
                }.into()
            }
            syn::Fields::Unit => {
                let ty = &input.ident;
                let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
                let mut where_clause = where_clause.cloned();
                add_bounds(&mut where_clause, &input.generics, &krate);
                quote! {
                    unsafe impl #impl_generics #krate::SerializeConst for #ty #ty_generics #where_clause {
                        const MEMORY_LAYOUT: #krate::Layout = #krate::Layout::Struct(#krate::StructLayout::new(
                            std::mem::size_of::<Self>(),
                            &[],
                        ));
                    }
                }.into()
            }
        },
        syn::Data::Enum(data) => match data.variants.len() {
            0 => syn::Error::new(input.ident.span(), "Enums must have at least one variant")
                .to_compile_error()
                .into(),
            1.. => {
                let mut repr_c = false;
                let mut discriminant_size = None;
                for attr in &input.attrs {
                    if attr.path().is_ident("repr") {
                        if let Err(err) = attr.parse_nested_meta(|meta| {
                            // #[repr(C)]
                            if meta.path.is_ident("C") {
                                repr_c = true;
                                return Ok(());
                            }

                            // #[repr(u8)]
                            if meta.path.is_ident("u8") {
                                discriminant_size = Some(1);
                                return Ok(());
                            }

                            // #[repr(u16)]
                            if meta.path.is_ident("u16") {
                                discriminant_size = Some(2);
                                return Ok(());
                            }

                            // #[repr(u32)]
                            if meta.path.is_ident("u32") {
                                discriminant_size = Some(3);
                                return Ok(());
                            }

                            // #[repr(u64)]
                            if meta.path.is_ident("u64") {
                                discriminant_size = Some(4);
                                return Ok(());
                            }

                            Err(meta.error("unrecognized repr"))
                        }) {
                            return err.to_compile_error().into();
                        }
                    }
                }

                let variants_have_fields = data
                    .variants
                    .iter()
                    .any(|variant| !variant.fields.is_empty());
                if !repr_c && variants_have_fields {
                    return syn::Error::new(input.ident.span(), "Enums must be repr(C, u*)")
                        .to_compile_error()
                        .into();
                }

                if discriminant_size.is_none() {
                    return syn::Error::new(input.ident.span(), "Enums must be repr(u*)")
                        .to_compile_error()
                        .into();
                }

                let ty = &input.ident;
                let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
                let mut where_clause = where_clause.cloned();
                add_bounds(&mut where_clause, &input.generics, &krate);
                let mut last_discriminant = None;
                let variants = data.variants.iter().map(|variant| {
                    let discriminant = variant
                        .discriminant
                        .as_ref()
                        .map(|(_, discriminant)| discriminant.to_token_stream())
                        .unwrap_or_else(|| match &last_discriminant {
                            Some(discriminant) => quote! { #discriminant + 1 },
                            None => {
                                quote! { 0 }
                            }
                        });
                    last_discriminant = Some(discriminant.clone());
                    let variant_name = &variant.ident;
                    let field_names = variant.fields.iter().enumerate().map(|(i, field)| {
                        field
                            .ident
                            .clone()
                            .unwrap_or_else(|| quote::format_ident!("__field_{}", i))
                    });
                    let field_types = variant.fields.iter().map(|field| &field.ty);
                    let generics = &input.generics;
                    quote! {
                        {
                            #[allow(unused)]
                            #[derive(#krate::SerializeConst)]
                            #[const_serialize(crate = #krate)]
                            #[repr(C)]
                            struct VariantStruct #generics {
                                #(
                                    #field_names: #field_types,
                                )*
                            }
                            #krate::EnumVariant::new(
                                stringify!(#variant_name),
                                #discriminant as u32,
                                match <VariantStruct #generics as #krate::SerializeConst>::MEMORY_LAYOUT {
                                    #krate::Layout::Struct(layout) => layout,
                                    _ => panic!("VariantStruct::MEMORY_LAYOUT must be a struct"),
                                },
                                ::std::mem::align_of::<VariantStruct>(),
                            )
                        }
                    }
                });
                quote! {
                    unsafe impl #impl_generics #krate::SerializeConst for #ty #ty_generics #where_clause {
                        const MEMORY_LAYOUT: #krate::Layout = #krate::Layout::Enum(#krate::EnumLayout::new(
                            ::std::mem::size_of::<Self>(),
                            #krate::PrimitiveLayout::new(
                                #discriminant_size as usize,
                            ),
                            {
                                const DATA: &'static [#krate::EnumVariant] = &[
                                    #(
                                        #variants,
                                    )*
                                ];
                                DATA
                            },
                        ));
                    }
                }.into()
            }
        },
        _ => syn::Error::new(input.ident.span(), "Only structs and enums are supported")
            .to_compile_error()
            .into(),
    }
}
