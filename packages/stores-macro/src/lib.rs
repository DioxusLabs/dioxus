use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote, ToTokens};
use syn::{
    parse_macro_input, parse_quote, punctuated::Punctuated, spanned::Spanned, DataStruct,
    DeriveInput, Fields, Index,
};

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
        syn::Data::Enum(_) => {
            return Err(syn::Error::new(
                input.span(),
                "Store macro does not yet support enums",
            ))
        }
        syn::Data::Union(_) => {
            return Err(syn::Error::new(
                input.span(),
                "Store macro does not support unions",
            ))
        }
    }
}

fn derive_store_struct(input: &DeriveInput, structure: &DataStruct) -> syn::Result<TokenStream2> {
    let struct_name = &input.ident;
    let fields = &structure.fields;
    let visibility = &input.vis;

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
    extension_map_bounds.push(
        parse_quote!(__W: dioxus_stores::macro_helpers::dioxus_signals::Writable<Target = #struct_name #ty_generics> + Copy + 'static),
    );
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

        let write_type = quote! { dioxus_stores::macro_helpers::dioxus_signals::MappedMutSignal<#field_type, __W> };

        let store_type = quote! { dioxus_stores::Store<#field_type, #write_type> };

        transposed_fields.push(quote! { #field_name #colon #store_type });

        let store_constructor = quote! { dioxus_stores::Store::new };

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
                let scope = self.selector().scope(
                    #ordinal,
                    __map_field,
                    __map_mut_field,
                );
                #store_constructor(scope)
            }
        };
        implementations.push(implementation);
    }

    let definition = quote! {
        fn transpose(
            self,
        ) -> #transposed_name #transposed_generics;
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
            self,
        ) -> #transposed_name #transposed_generics {
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
        #visibility trait #extension_trait_name<__W> where #extension_map_bounds {
            #(
                #definitions
            )*
        }


        impl #extension_impl_generics #extension_trait_name<__W> for dioxus_stores::Store<#struct_name, __W> #ty_generics #extension_map_where_clause {
            #(
                #implementations
            )*
        }

        #transposed_struct
    };

    Ok(expanded)
}
