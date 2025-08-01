use convert_case::{Case, Casing};
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote, ToTokens};
use syn::{
    parse_macro_input, parse_quote, punctuated::Punctuated, spanned::Spanned, DataEnum, DataStruct,
    DeriveInput, Fields, Index,
};

/// # `derive(Store)`
///
/// The `Store` macro is used to create an extension trait for stores that makes it possible to access the fields or variants
/// of an item as stores.
///
/// ## Expansion
///
/// The macro expands to two different items:
/// - An extension trait which is implemented for `Store<YourType, W>` with methods to access fields and variants for your type.
/// - A transposed version of your type which contains the fields or variants as stores.
///
/// ### Structs
///
/// For structs, the store macro generates methods for each field that returns a store scoped to that field and a `transpose` method that returns a struct with all fields as stores:
///
/// ```rust, no_run
/// use dioxus::prelude::*;
/// use dioxus_stores::*;
///
/// #[derive(Store)]
/// struct TodoItem {
///     checked: bool,
///     contents: String,
/// }
///
/// let store = use_store(|| TodoItem {
///     checked: false,
///     contents: "Learn about stores".to_string(),
/// });
///
/// // The store macro creates an extension trait with methods for each field
/// // that returns a store scoped to that field.
/// let checked: Store<bool, _> = store.checked();
/// let contents: Store<String, _> = store.contents();
///
/// // It also generates a `transpose` method returns a variant of your structure
/// // with stores wrapping each of your data types. This can be very useful when destructuring
/// // or matching your type
/// let TodoItemStoreTransposed { checked, contents } = store.transpose();
/// let checked: bool = checked();
/// let contents: String = contents();
/// ```
///
///
/// ### Enums
///
/// For enums, the store macro generates methods for each variant that checks if the store is that variant. It also generates a `transpose` method that returns an enum with all fields as stores.
///
/// ```rust, no_run
/// use dioxus::prelude::*;
/// use dioxus_stores::*;
///
/// #[derive(Store, PartialEq, Clone, Debug)]
/// enum Enum {
///     Foo(String),
///     Bar { foo: i32, bar: String },
/// }
///
/// let store = use_store(|| Enum::Foo("Hello".to_string()));
/// // The store macro creates an extension trait with methods for each variant to check
/// // if the store is that variant.
/// let foo: bool = store.is_foo();
/// let bar: bool = store.is_bar();
///
/// // If there is only one field in the variant, it also generates a method to try
/// // to downcast the store to that variant.
/// let foo: Option<Store<String, _>> = store.foo();
/// if let Some(foo) = foo {
///     println!("Foo: {foo}");
/// }
///
/// // It also generates a `transpose` method that returns a variant of your enum where all
/// // the fields are stores. You can use this to match your enum
/// let transposed = store.transpose();
/// use EnumStoreTransposed::*;
/// match transposed {
///     EnumStoreTransposed::Foo(foo) => println!("Foo: {foo}"),
///     EnumStoreTransposed::Bar { foo, bar } => {
///         let foo: i32 = foo();
///         let bar: String = bar();
///         println!("Bar: foo = {foo}, bar = {bar}");
///     }
/// }
/// ```
#[proc_macro_derive(Store)]
pub fn store(input: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let input = parse_macro_input!(input as DeriveInput);

    let expanded = match derive_store(input) {
        Ok(tokens) => tokens,
        Err(err) => {
            // If there was an error, return it as a compile error
            return err.to_compile_error().into();
        }
    };

    // Hand the output tokens back to the compiler
    TokenStream::from(expanded)
}

fn derive_store(input: DeriveInput) -> syn::Result<TokenStream2> {
    match &input.data {
        syn::Data::Struct(data_struct) => derive_store_struct(&input, data_struct),
        syn::Data::Enum(data_enum) => derive_store_enum(&input, data_enum),
        syn::Data::Union(_) => Err(syn::Error::new(
            input.span(),
            "Store macro does not support unions",
        )),
    }
}

fn derive_store_struct(input: &DeriveInput, structure: &DataStruct) -> syn::Result<TokenStream2> {
    let struct_name = &input.ident;
    let fields = &structure.fields;
    let visibility = &input.vis;

    // We don't need to do anything if there are no fields
    if fields.is_empty() {
        return Ok(quote! {});
    }

    let extension_trait_name = format_ident!("{}StoreExt", struct_name);
    let transposed_name = format_ident!("{}StoreTransposed", struct_name);

    let generics = &input.generics;
    let (_, ty_generics, _) = generics.split_for_impl();

    // Extend the original generics with a view and storage type for the selector generics
    let mut extension_generics = generics.clone();
    extension_generics.params.insert(0, parse_quote!(__W));

    let (extension_impl_generics, transposed_generics, _) = extension_generics.split_for_impl();

    let mut extension_map_bounds: Punctuated<syn::WherePredicate, syn::Token![,]> =
        Punctuated::new();
    for generic in generics.type_params() {
        let ident = &generic.ident;
        extension_map_bounds.push(parse_quote!(#ident: 'static));
    }
    let extension_map_where_clause = if let Some(mut clause) = generics.where_clause.clone() {
        clause
            .predicates
            .extend(extension_map_bounds.iter().cloned());
        clause.into_token_stream()
    } else {
        quote! { where #extension_map_bounds }
    };

    let mut implementations = Vec::new();
    let mut definitions = Vec::new();
    let mut transposed_fields = Vec::new();

    for (i, field) in fields.iter().enumerate() {
        let vis = &field.vis;
        let field_name = &field.ident;
        let colon = field.colon_token.as_ref();
        let field_accessor = field_name.as_ref().map_or_else(
            || Index::from(i).to_token_stream(),
            |name| name.to_token_stream(),
        );
        let function_name = field_name
            .as_ref()
            .map_or_else(|| format_ident!("field_{i}"), |name| name.clone());
        let field_type = &field.ty;

        let write_type = quote! { dioxus_stores::macro_helpers::dioxus_signals::MappedMutSignal<#field_type, __W, fn(&#struct_name #ty_generics) -> &#field_type,  fn(&mut #struct_name #ty_generics) -> &mut #field_type> };

        let store_type = quote! { dioxus_stores::Store<#field_type, #write_type> };

        transposed_fields.push(quote! { #vis #field_name #colon #store_type });

        let ordinal = i as u32;

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
                let scope = self.into_selector().child(
                    #ordinal,
                    __map_field,
                    __map_mut_field,
                );
                ::std::convert::Into::into(scope)
            }
        };
        implementations.push(implementation);
    }

    let definition = quote! {
        fn transpose(
            &self,
        ) -> #transposed_name #transposed_generics where Self: ::std::marker::Copy;
    };
    definitions.push(definition);
    let field_names = fields
        .iter()
        .enumerate()
        .map(|(i, field)| {
            field
                .ident
                .as_ref()
                .map_or_else(|| format_ident!("field_{i}"), |name| name.clone())
        })
        .collect::<Vec<_>>();
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
            &self,
        ) -> #transposed_name #transposed_generics where Self: ::std::marker::Copy {
            #(
                let #field_names = self.#field_names();
            )*
            #construct
        }
    };
    implementations.push(implementation);

    let transposed_struct = match &structure.fields {
        Fields::Named(_) => {
            quote! { #visibility struct #transposed_name #transposed_generics #extension_map_where_clause {#(#transposed_fields),*} }
        }
        Fields::Unnamed(_) => {
            quote! { #visibility struct #transposed_name #transposed_generics (#(#transposed_fields),*) #extension_map_where_clause; }
        }
        Fields::Unit => {
            quote! {#visibility struct #transposed_name #transposed_generics #extension_map_where_clause;}
        }
    };

    // Generate the store implementation
    let expanded = quote! {
        #visibility trait #extension_trait_name #transposed_generics  where #extension_map_bounds {
            #(
                #definitions
            )*
        }

        impl #extension_impl_generics #extension_trait_name #transposed_generics for dioxus_stores::Store<#struct_name #ty_generics, __W> #extension_map_where_clause {
            #(
                #implementations
            )*
        }

        #transposed_struct
    };

    Ok(expanded)
}

fn derive_store_enum(input: &DeriveInput, structure: &DataEnum) -> syn::Result<TokenStream2> {
    let enum_name = &input.ident;
    let variants = &structure.variants;
    let visibility = &input.vis;

    let extension_trait_name = format_ident!("{}StoreExt", enum_name);
    let transposed_name = format_ident!("{}StoreTransposed", enum_name);

    let generics = &input.generics;
    let (_, ty_generics, _) = generics.split_for_impl();

    // Extend the original generics with a view and storage type for the selector generics
    let mut extension_generics = generics.clone();
    extension_generics.params.insert(0, parse_quote!(__W));

    let (extension_impl_generics, transposed_generics, _) = extension_generics.split_for_impl();

    let mut extension_map_bounds: Punctuated<syn::WherePredicate, syn::Token![,]> =
        Punctuated::new();
    for generic in generics.type_params() {
        let ident = &generic.ident;
        extension_map_bounds.push(parse_quote!(#ident: 'static));
    }
    let extension_map_where_clause = if let Some(mut clause) = generics.where_clause.clone() {
        clause
            .predicates
            .extend(extension_map_bounds.iter().cloned());
        clause.into_token_stream()
    } else {
        quote! { where #extension_map_bounds }
    };

    let mut implementations = Vec::new();
    let mut definitions = Vec::new();
    let mut transposed_variants = Vec::new();
    let mut transposed_match_arms = Vec::new();

    for variant in variants {
        let variant_name = &variant.ident;
        let snake_case_variant = format_ident!("{}", variant_name.to_string().to_case(Case::Snake));
        let is_fn = format_ident!("is_{}", snake_case_variant);
        let definition = quote! {
            fn #is_fn(
                self,
            ) -> bool;
        };
        definitions.push(definition);
        let implementation = quote! {
            fn #is_fn(
                self,
            ) -> bool where Self: dioxus_stores::macro_helpers::dioxus_signals::Readable<Target = #enum_name #ty_generics> {
                self.selector().track_shallow();
                let ref_self = dioxus_stores::macro_helpers::dioxus_signals::ReadableExt::peek(self.selector());
                matches!(&*ref_self, #enum_name::#variant_name { .. })
            }
        };
        implementations.push(implementation);

        let mut transposed_fields = Vec::new();
        let mut transposed_field_selectors = Vec::new();
        let fields = &variant.fields;
        for (i, field) in fields.iter().enumerate() {
            let ordinal = i as u32;
            let vis = &field.vis;
            let field_name = &field.ident;
            let colon = field.colon_token.as_ref();
            let function_name = field_name
                .as_ref()
                .map_or_else(|| format_ident!("field_{i}"), |name| name.clone());
            let field_type = &field.ty;

            let write_type = quote! { dioxus_stores::macro_helpers::dioxus_signals::MappedMutSignal<#field_type, __W, fn(&#enum_name #ty_generics) -> &#field_type, fn(&mut #enum_name #ty_generics) -> &mut #field_type> };

            let store_type = quote! { dioxus_stores::Store<#field_type, #write_type> };

            transposed_fields.push(quote! { #vis #field_name #colon #store_type });
            let match_field = if field_name.is_none() {
                let ignore_before = (0..i).map(|_| quote!(_));
                let ignore_after = (i + 1..fields.len()).map(|_| quote!(_));
                quote!( ( #(#ignore_before,)* #function_name, #(#ignore_after),* ) )
            } else {
                quote!( { #function_name, .. })
            };
            let select_field = quote! {
                let __map_field: fn(&#enum_name #ty_generics) -> &#field_type = |value| match value {
                    #enum_name::#variant_name #match_field => #function_name,
                    _ => panic!("Selector that was created to match {} read after variant changed", stringify!(#variant_name)),
                };
                let __map_mut_field: fn(&mut #enum_name #ty_generics) -> &mut #field_type = |value| match value {
                    #enum_name::#variant_name #match_field => #function_name,
                    _ => panic!("Selector that was created to match {} written after variant changed", stringify!(#variant_name)),
                };
                let scope = self.into_selector().child(
                    #ordinal,
                    __map_field,
                    __map_mut_field,
                );
                ::std::convert::Into::into(scope)
            };

            // If there is only one field, generate a field() -> Option<Store<O, W>> method
            if fields.len() == 1 {
                let definition = quote! {
                    fn #snake_case_variant(
                        self,
                    ) -> Option<#store_type>;
                };
                definitions.push(definition);
                let implementation = quote! {
                    fn #snake_case_variant(
                        self,
                    ) -> Option<#store_type> {
                        self.#is_fn().then(|| {
                            #select_field
                        })
                    }
                };
                implementations.push(implementation);
            }

            transposed_field_selectors.push(select_field);
        }

        let field_names = fields
            .iter()
            .enumerate()
            .map(|(i, field)| {
                field
                    .ident
                    .as_ref()
                    .map_or_else(|| format_ident!("field_{i}"), |name| name.clone())
            })
            .collect::<Vec<_>>();
        let construct_fields = field_names
            .iter()
            .zip(transposed_field_selectors.iter())
            .map(|(name, selector)| {
                quote! { let #name = { #selector }; }
            });
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

        let transposed_variant = match &fields {
            Fields::Named(_) => {
                quote! { #variant_name {#(#transposed_fields),*} }
            }
            Fields::Unnamed(_) => {
                quote! { #variant_name (#(#transposed_fields),*) }
            }
            Fields::Unit => {
                quote! { #variant_name }
            }
        };
        transposed_variants.push(transposed_variant);
    }

    let definition = quote! {
        fn transpose(
            &self,
        ) -> #transposed_name #transposed_generics where Self: ::std::marker::Copy;
    };
    definitions.push(definition);
    let implementation = quote! {
        fn transpose(
            &self,
        ) -> #transposed_name #transposed_generics where Self: dioxus_stores::macro_helpers::dioxus_signals::Readable<Target = #enum_name #ty_generics> + ::std::marker::Copy {
            self.selector().track_shallow();
            let read = dioxus_stores::macro_helpers::dioxus_signals::ReadableExt::peek(self.selector());
            match &*read {
                #(#transposed_match_arms)*
                #[allow(unreachable)]
                _ => unreachable!(),
            }
        }
    };
    implementations.push(implementation);

    let transposed_enum = quote! { #visibility enum #transposed_name #transposed_generics #extension_map_where_clause {#(#transposed_variants),*} };

    // Generate the store implementation
    let expanded = quote! {
        #visibility trait #extension_trait_name #transposed_generics where #extension_map_bounds {
            #(
                #definitions
            )*
        }

        impl #extension_impl_generics #extension_trait_name #transposed_generics for dioxus_stores::Store<#enum_name #ty_generics, __W> #extension_map_where_clause {
            #(
                #implementations
            )*
        }

        #transposed_enum
    };

    Ok(expanded)
}
