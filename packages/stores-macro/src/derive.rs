use convert_case::{Case, Casing};
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote, ToTokens};
use syn::{
    parse_quote, spanned::Spanned, DataEnum, DataStruct, DeriveInput, Field, Fields, Generics,
    Ident, Index, LitInt,
};

pub(crate) fn derive_store(input: DeriveInput) -> syn::Result<TokenStream2> {
    let item_name = &input.ident;
    let extension_trait_name = format_ident!("{}StoreExt", item_name);
    let transposed_name = format_ident!("{}StoreTransposed", item_name);

    // Create generics for the extension trait and transposed struct. Both items need the original generics
    // and bounds plus an extra __Lens type used in the store generics
    let generics = &input.generics;
    let mut extension_generics = generics.clone();
    extension_generics.params.insert(0, parse_quote!(__Lens));

    match &input.data {
        syn::Data::Struct(data_struct) => derive_store_struct(
            &input,
            data_struct,
            extension_trait_name,
            transposed_name,
            extension_generics,
        ),
        syn::Data::Enum(data_enum) => derive_store_enum(
            &input,
            data_enum,
            extension_trait_name,
            transposed_name,
            extension_generics,
        ),
        syn::Data::Union(_) => Err(syn::Error::new(
            input.span(),
            "Store macro does not support unions",
        )),
    }
}

// For structs, we derive two items:
// - An extension trait with methods to access the fields of the struct as stores and a `transpose` method
// - A transposed version of the struct with all fields wrapped in stores
fn derive_store_struct(
    input: &DeriveInput,
    structure: &DataStruct,
    extension_trait_name: Ident,
    transposed_name: Ident,
    extension_generics: Generics,
) -> syn::Result<TokenStream2> {
    let struct_name = &input.ident;
    let fields = &structure.fields;
    let visibility = &input.vis;

    // We don't need to do anything if there are no fields
    if fields.is_empty() {
        return Ok(quote! {});
    }

    let generics = &input.generics;
    let (_, ty_generics, _) = generics.split_for_impl();
    let (extension_impl_generics, extension_ty_generics, extension_where_clause) =
        extension_generics.split_for_impl();

    // We collect the definitions and implementations for the extension trait methods along with the types of the fields in the transposed struct
    let mut implementations = Vec::new();
    let mut definitions = Vec::new();
    let mut transposed_fields = Vec::new();

    for (field_index, field) in fields.iter().enumerate() {
        generate_field_methods(
            field_index,
            field,
            struct_name,
            &ty_generics,
            &mut transposed_fields,
            &mut definitions,
            &mut implementations,
        );
    }

    // Add a transpose method to turn the stored struct into a struct with all fields as stores
    // We need the copy bound here because the store will be copied into the selector for each field
    let definition = quote! {
        fn transpose(
            self,
        ) -> #transposed_name #extension_ty_generics where Self: ::std::marker::Copy;
    };
    definitions.push(definition);
    let field_names = fields
        .iter()
        .enumerate()
        .map(|(i, field)| function_name_from_field(i, field))
        .collect::<Vec<_>>();
    // Construct the transposed struct with the fields as stores from the field variables in scope
    let construct = match &structure.fields {
        Fields::Named(_) => {
            quote! { #transposed_name { #(#field_names),* } }
        }
        Fields::Unnamed(_) => {
            quote! { #transposed_name(#(#field_names),*) }
        }
        Fields::Unit => {
            quote! { #transposed_name }
        }
    };
    let implementation = quote! {
        fn transpose(
            self,
        ) -> #transposed_name #extension_ty_generics where Self: ::std::marker::Copy {
            // Convert each field into the corresponding store
            #(
                let #field_names = self.#field_names();
            )*
            #construct
        }
    };
    implementations.push(implementation);

    // Generate the transposed struct definition
    let transposed_struct = transposed_struct(
        visibility,
        struct_name,
        &transposed_name,
        structure,
        generics,
        &extension_generics,
        &transposed_fields,
    );

    // Expand to the extension trait and its implementation for the store alongside the transposed struct
    Ok(quote! {
        #visibility trait #extension_trait_name #extension_impl_generics #extension_where_clause {
            #(
                #definitions
            )*
        }

        impl #extension_impl_generics #extension_trait_name #extension_ty_generics for dioxus_stores::Store<#struct_name #ty_generics, __Lens> #extension_where_clause {
            #(
                #implementations
            )*
        }

        #transposed_struct
    })
}

fn field_type_generic(field: &Field, generics: &syn::Generics) -> bool {
    generics.type_params().any(|param| {
        matches!(&field.ty, syn::Type::Path(type_path) if type_path.path.is_ident(&param.ident))
    })
}

fn transposed_struct(
    visibility: &syn::Visibility,
    struct_name: &Ident,
    transposed_name: &Ident,
    structure: &DataStruct,
    generics: &syn::Generics,
    extension_generics: &syn::Generics,
    transposed_fields: &[TokenStream2],
) -> TokenStream2 {
    let (extension_impl_generics, _, extension_where_clause) = extension_generics.split_for_impl();
    // Only use a type alias if:
    // - There are no bounds on the type generics
    // - All fields are generic types
    let use_type_alias = generics.type_params().all(|param| param.bounds.is_empty())
        && structure
            .fields
            .iter()
            .all(|field| field_type_generic(field, generics));
    if use_type_alias {
        let generics = transpose_generics(struct_name, generics);
        return quote! {#visibility type #transposed_name #extension_impl_generics = #struct_name #generics;};
    }
    match &structure.fields {
        Fields::Named(fields) => {
            let fields = fields.named.iter();
            let fields = fields.zip(transposed_fields.iter()).map(|(f, t)| {
                let vis = &f.vis;
                let ident = &f.ident;
                let colon = f.colon_token.as_ref();
                quote! { #vis #ident #colon #t }
            });
            quote! {
                #visibility struct #transposed_name #extension_impl_generics #extension_where_clause {
                    #(
                        #fields
                    ),*
                }
            }
        }
        Fields::Unnamed(fields) => {
            let fields = fields.unnamed.iter();
            let fields = fields.zip(transposed_fields.iter()).map(|(f, t)| {
                let vis = &f.vis;
                quote! { #vis #t }
            });
            quote! {
                #visibility struct #transposed_name #extension_impl_generics (
                    #(
                        #fields
                    ),*
                )
                #extension_where_clause;
            }
        }
        Fields::Unit => {
            quote! {#visibility struct #transposed_name #extension_impl_generics #extension_where_clause}
        }
    }
}

fn generate_field_methods(
    field_index: usize,
    field: &syn::Field,
    struct_name: &Ident,
    ty_generics: &syn::TypeGenerics,
    transposed_fields: &mut Vec<TokenStream2>,
    definitions: &mut Vec<TokenStream2>,
    implementations: &mut Vec<TokenStream2>,
) {
    let field_name = &field.ident;

    // When we map the field, we need to use either the field name for named fields or the index for unnamed fields.
    let field_accessor = field_name.as_ref().map_or_else(
        || Index::from(field_index).to_token_stream(),
        |name| name.to_token_stream(),
    );
    let function_name = function_name_from_field(field_index, field);
    let field_type = &field.ty;
    let store_type = mapped_type(struct_name, ty_generics, field_type);

    transposed_fields.push(store_type.clone());

    // Each field gets its own reactive scope within the child based on the field's index
    let ordinal = LitInt::new(&field_index.to_string(), field.span());

    let definition = quote! {
        fn #function_name(
            self,
        ) -> #store_type;
    };
    definitions.push(definition);
    let implementation = quote! {
        fn #function_name(
            self,
        ) -> #store_type {
            let __map_field: fn(&#struct_name #ty_generics) -> &#field_type = |value| &value.#field_accessor;
            let __map_mut_field: fn(&mut #struct_name #ty_generics) -> &mut #field_type = |value| &mut value.#field_accessor;
            // Map the field into a child selector that tracks the field
            let scope = self.into_selector().child(
                #ordinal,
                __map_field,
                __map_mut_field,
            );
            // Convert the selector into a store
            ::std::convert::Into::into(scope)
        }
    };
    implementations.push(implementation);
}

// For enums, we derive two items:
// - An extension trait with methods to check if the store is a specific variant and a method
//   to access the field of that variant if there is only one field
// - A transposed version of the enum with all fields wrapped in stores
fn derive_store_enum(
    input: &DeriveInput,
    structure: &DataEnum,
    extension_trait_name: Ident,
    transposed_name: Ident,
    extension_generics: Generics,
) -> syn::Result<TokenStream2> {
    let enum_name = &input.ident;
    let variants = &structure.variants;
    let visibility = &input.vis;

    let generics = &input.generics;
    let (_, ty_generics, _) = generics.split_for_impl();
    let (extension_impl_generics, extension_ty_generics, extension_where_clause) =
        extension_generics.split_for_impl();

    // We collect the definitions and implementations for the extension trait methods along with the types of the fields in the transposed enum
    // and the match arms for the transposed enum.
    let mut implementations = Vec::new();
    let mut definitions = Vec::new();
    let mut transposed_variants = Vec::new();
    let mut transposed_match_arms = Vec::new();

    // The generated items that check the variant of the enum need to read the enum which requires these extra bounds
    let readable_bounds = quote! { __Lens: dioxus_stores::macro_helpers::dioxus_signals::Readable<Target = #enum_name #ty_generics>, #enum_name #ty_generics: 'static };

    for variant in variants {
        let variant_name = &variant.ident;
        let snake_case_variant = format_ident!("{}", variant_name.to_string().to_case(Case::Snake));
        let is_fn = format_ident!("is_{}", snake_case_variant);

        generate_is_variant_method(
            &is_fn,
            variant_name,
            enum_name,
            readable_bounds.clone(),
            &mut definitions,
            &mut implementations,
        );

        let mut transposed_fields = Vec::new();
        let mut transposed_field_selectors = Vec::new();
        let fields = &variant.fields;
        for (i, field) in fields.iter().enumerate() {
            let field_type = &field.ty;
            let store_type = mapped_type(enum_name, &ty_generics, field_type);

            // Push the field for the transposed enum
            transposed_fields.push(store_type.clone());

            // Generate the code to get Store<Field, W> from the enum
            let select_field = select_enum_variant_field(
                enum_name,
                &ty_generics,
                variant_name,
                field,
                i,
                fields.len(),
            );

            // If there is only one field, generate a field() -> Option<Store<O, W>> method
            if fields.len() == 1 {
                generate_as_variant_method(
                    &is_fn,
                    &snake_case_variant,
                    &select_field,
                    &store_type,
                    &readable_bounds,
                    &mut definitions,
                    &mut implementations,
                );
            }

            transposed_field_selectors.push(select_field);
        }

        // Now that we have the types for the field selectors within the variant,
        // we can construct the transposed variant and the logic to turn the normal
        // version of that variant into the store version
        let field_names = fields
            .iter()
            .enumerate()
            .map(|(i, field)| function_name_from_field(i, field))
            .collect::<Vec<_>>();
        // Turn each field into its store
        let construct_fields = field_names
            .iter()
            .zip(transposed_field_selectors.iter())
            .map(|(name, selector)| {
                quote! { let #name = { #selector }; }
            });
        // Merge the stores into the variant
        let construct_variant = match &fields {
            Fields::Named(_) => {
                quote! { #transposed_name::#variant_name { #(#field_names),* } }
            }
            Fields::Unnamed(_) => {
                quote! { #transposed_name::#variant_name(#(#field_names),*) }
            }
            Fields::Unit => {
                quote! { #transposed_name::#variant_name }
            }
        };
        let match_arm = quote! {
            #enum_name::#variant_name { .. } => {
                #(#construct_fields)*
                #construct_variant
            },
        };
        transposed_match_arms.push(match_arm);

        // Push the type definition of the variant to the transposed enum
        let transposed_variant = match &fields {
            Fields::Named(named) => {
                let fields = named.named.iter();
                let fields = fields.zip(transposed_fields.iter()).map(|(f, t)| {
                    let vis = &f.vis;
                    let ident = &f.ident;
                    let colon = f.colon_token.as_ref();
                    quote! { #vis #ident #colon #t }
                });
                quote! { #variant_name {
                    #(
                        #fields
                    ),*
                } }
            }
            Fields::Unnamed(unnamed) => {
                let fields = unnamed.unnamed.iter();
                let fields = fields.zip(transposed_fields.iter()).map(|(f, t)| {
                    let vis = &f.vis;
                    quote! { #vis #t }
                });
                quote! { #variant_name (
                    #(
                        #fields
                    ),*
                ) }
            }
            Fields::Unit => {
                quote! { #variant_name }
            }
        };
        transposed_variants.push(transposed_variant);
    }

    let definition = quote! {
        fn transpose(
            self,
        ) -> #transposed_name #extension_ty_generics where #readable_bounds, Self: ::std::marker::Copy;
    };
    definitions.push(definition);
    let implementation = quote! {
        fn transpose(
            self,
        ) -> #transposed_name #extension_ty_generics where #readable_bounds, Self: ::std::marker::Copy {
            // We only do a shallow read of the store to get the current variant. We only need to rerun
            // this match when the variant changes, not when the fields change
            self.selector().track_shallow();
            let read = dioxus_stores::macro_helpers::dioxus_signals::ReadableExt::peek(self.selector());
            match &*read {
                #(#transposed_match_arms)*
                // The enum may be #[non_exhaustive]
                #[allow(unreachable)]
                _ => unreachable!(),
            }
        }
    };
    implementations.push(implementation);

    // Only use a type alias if:
    // - There are no bounds on the type generics
    // - All fields are generic types
    let use_type_alias = generics.type_params().all(|param| param.bounds.is_empty())
        && structure
            .variants
            .iter()
            .flat_map(|variant| variant.fields.iter())
            .all(|field| field_type_generic(field, generics));

    let transposed_enum = if use_type_alias {
        let generics = transpose_generics(enum_name, generics);

        quote! {#visibility type #transposed_name #extension_generics = #enum_name #generics;}
    } else {
        quote! { #visibility enum #transposed_name #extension_impl_generics #extension_where_clause {#(#transposed_variants),*} }
    };

    // Expand to the extension trait and its implementation for the store alongside the transposed enum
    Ok(quote! {
        #visibility trait #extension_trait_name #extension_impl_generics #extension_where_clause {
            #(
                #definitions
            )*
        }

        impl #extension_impl_generics #extension_trait_name #extension_ty_generics for dioxus_stores::Store<#enum_name #ty_generics, __Lens> #extension_where_clause {
            #(
                #implementations
            )*
        }

        #transposed_enum
    })
}

fn generate_is_variant_method(
    is_fn: &Ident,
    variant_name: &Ident,
    enum_name: &Ident,
    readable_bounds: TokenStream2,
    definitions: &mut Vec<TokenStream2>,
    implementations: &mut Vec<TokenStream2>,
) {
    // Generate a is_variant method that checks if the store is a specific variant
    let definition = quote! {
        fn #is_fn(
            &self,
        ) -> bool where #readable_bounds;
    };
    definitions.push(definition);
    let implementation = quote! {
        fn #is_fn(
            &self,
        ) -> bool where #readable_bounds {
            // Reading the current variant only tracks the shallow value of the store. Writing to a specific
            // variant will not cause the variant to change, so we don't need to subscribe deeply
            self.selector().track_shallow();
            let ref_self = dioxus_stores::macro_helpers::dioxus_signals::ReadableExt::peek(self.selector());
            matches!(&*ref_self, #enum_name::#variant_name { .. })
        }
    };
    implementations.push(implementation);
}

/// Generate a method to turn Store<Enum, W> into Option<Store<VariantField, W>> if the variant only has one field.
fn generate_as_variant_method(
    is_fn: &Ident,
    snake_case_variant: &Ident,
    select_field: &TokenStream2,
    store_type: &TokenStream2,
    readable_bounds: &TokenStream2,
    definitions: &mut Vec<TokenStream2>,
    implementations: &mut Vec<TokenStream2>,
) {
    let definition = quote! {
        fn #snake_case_variant(
            self,
        ) -> Option<#store_type> where #readable_bounds;
    };
    definitions.push(definition);
    let implementation = quote! {
        fn #snake_case_variant(
            self,
        ) -> Option<#store_type> where #readable_bounds {
            self.#is_fn().then(|| {
                #select_field
            })
        }
    };
    implementations.push(implementation);
}

fn select_enum_variant_field(
    enum_name: &Ident,
    ty_generics: &syn::TypeGenerics,
    variant_name: &Ident,
    field: &Field,
    field_index: usize,
    field_count: usize,
) -> TokenStream2 {
    // Generate the match arm for the field
    let function_name = function_name_from_field(field_index, field);
    let field_type = &field.ty;
    let match_field = if field.ident.is_none() {
        let ignore_before = (0..field_index).map(|_| quote!(_));
        let ignore_after = (field_index + 1..field_count).map(|_| quote!(_));
        quote!( ( #(#ignore_before,)* #function_name, #(#ignore_after),* ) )
    } else {
        quote!( { #function_name, .. })
    };
    let ordinal = LitInt::new(&field_index.to_string(), variant_name.span());
    quote! {
        let __map_field: fn(&#enum_name #ty_generics) -> &#field_type = |value| match value {
            #enum_name::#variant_name #match_field => #function_name,
            _ => panic!("Selector that was created to match {} read after variant changed", stringify!(#variant_name)),
        };
        let __map_mut_field: fn(&mut #enum_name #ty_generics) -> &mut #field_type = |value| match value {
            #enum_name::#variant_name #match_field => #function_name,
            _ => panic!("Selector that was created to match {} written after variant changed", stringify!(#variant_name)),
        };
        // Each field within the variant gets its own reactive scope. Writing to one field will not notify the enum or
        // other fields
        let scope = self.into_selector().child(
            #ordinal,
            __map_field,
            __map_mut_field,
        );
        ::std::convert::Into::into(scope)
    }
}

fn function_name_from_field(index: usize, field: &syn::Field) -> Ident {
    // Generate a function name from the field's identifier or index
    field
        .ident
        .as_ref()
        .map_or_else(|| format_ident!("field_{index}"), |name| name.clone())
}

fn mapped_type(
    item: &Ident,
    ty_generics: &syn::TypeGenerics,
    field_type: &syn::Type,
) -> TokenStream2 {
    // The zoomed in store type is a MappedMutSignal with function pointers to map the reference to the enum into a reference to the field
    let write_type = quote! { dioxus_stores::macro_helpers::dioxus_signals::MappedMutSignal<#field_type, __Lens, fn(&#item #ty_generics) -> &#field_type, fn(&mut #item #ty_generics) -> &mut #field_type> };
    quote! { dioxus_stores::Store<#field_type, #write_type> }
}

/// Take the generics from the original type with only generic fields into the generics for the transposed type
fn transpose_generics(name: &Ident, generics: &syn::Generics) -> TokenStream2 {
    let (_, ty_generics, _) = generics.split_for_impl();
    let mut transposed_generics = generics.clone();
    let mut generics = Vec::new();
    for gen in transposed_generics.params.iter_mut() {
        match gen {
            // Map type generics into Store<Type, MappedMutSignal<...>>
            syn::GenericParam::Type(type_param) => {
                let ident = &type_param.ident;
                let ty = mapped_type(name, &ty_generics, &parse_quote!(#ident));
                generics.push(ty);
            }
            // Forward const and lifetime generics as-is
            syn::GenericParam::Const(const_param) => {
                let ident = &const_param.ident;
                generics.push(quote! { #ident });
            }
            syn::GenericParam::Lifetime(lt_param) => {
                let ident = &lt_param.lifetime;
                generics.push(quote! { #ident });
            }
        }
    }

    quote!(<#(#generics),*> )
}
