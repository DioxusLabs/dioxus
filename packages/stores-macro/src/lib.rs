use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote, ToTokens};
use syn::{
    parse::Parse, parse_macro_input, parse_quote, punctuated::Punctuated, spanned::Spanned,
    AngleBracketedGenericArguments, DataStruct, DeriveInput, Ident, Index,
};

#[proc_macro_derive(Store, attributes(store))]
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

    let selector_name = format_ident!("{}Selector", struct_name);

    let generics = &input.generics;
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    // Extend the original generics with a view and storage type for the selector generics
    let mut selector_generics = generics.clone();
    selector_generics.params.push(parse_quote!(__W));

    let (selector_impl_generics, selector_ty_generics, selector_where_clause) =
        selector_generics.split_for_impl();

    let mut selector_map_bounds: Punctuated<syn::WherePredicate, syn::Token![,]> =
        Punctuated::new();
    selector_map_bounds.push(
        parse_quote!(__W: dioxus_stores::macro_helpers::dioxus_signals::Writable<Target = #struct_name #ty_generics> + Copy + 'static),
    );
    for generic in generics.type_params() {
        let ident = &generic.ident;
        selector_map_bounds.push(parse_quote!(#ident: 'static));
    }
    let selector_map_where_clause = if let Some(mut clause) = generics.where_clause.clone() {
        clause.predicates.extend(selector_map_bounds);
        clause.into_token_stream()
    } else {
        quote! { where #selector_map_bounds }
    };

    let mut selector_clone_bounds: Punctuated<syn::WherePredicate, syn::Token![,]> =
        Punctuated::new();
    selector_clone_bounds.push(parse_quote!(__W: ::std::clone::Clone));
    let selector_clone_where_clause = if let Some(mut clause) = generics.where_clause.clone() {
        clause.predicates.extend(selector_clone_bounds);
        clause.into_token_stream()
    } else {
        quote! { where #selector_clone_bounds }
    };

    let mut selector_copy_bounds: Punctuated<syn::WherePredicate, syn::Token![+]> =
        Punctuated::new();
    selector_copy_bounds.push(parse_quote!(__W: ::std::marker::Copy));
    let selector_copy_where_clause = if let Some(mut clause) = generics.where_clause.clone() {
        clause.predicates.extend(selector_copy_bounds);
        clause.into_token_stream()
    } else {
        quote! { where #selector_copy_bounds }
    };

    let mut selector_partial_eq_bounds: Punctuated<syn::WherePredicate, syn::Token![,]> =
        Punctuated::new();
    selector_partial_eq_bounds.push(parse_quote!(__W: ::std::cmp::PartialEq));
    let selector_partial_eq_where_clause = if let Some(mut clause) = generics.where_clause.clone() {
        clause.predicates.extend(selector_partial_eq_bounds);
        clause.into_token_stream()
    } else {
        quote! { where #selector_partial_eq_bounds }
    };

    let store_struct_into_boxed = derive_store_struct_into_boxed(input, &selector_name)?;
    let store_struct_readable = derive_store_struct_readable(input, &selector_name)?;
    let store_struct_writable = derive_store_struct_writable(input, &selector_name)?;

    let fields = fields
        .iter()
        .enumerate()
        .map(|(i, field)| {
            let field_name = &field.ident;
            let parsed_attributes = field
                .attrs
                .iter()
                .filter_map(StoreAttribute::from_attribute)
                .collect::<Result<Vec<_>, _>>()?;
            let foreign = parsed_attributes
                .iter()
                .any(|attr| matches!(attr, StoreAttribute::Foreign));
            let field_accessor = field_name.as_ref().map_or_else(
                || Index::from(i).to_token_stream(),
                |name| name.to_token_stream(),
            );
            let function_name = field_name
                .as_ref()
                .map_or_else(|| format_ident!("field_{i}"), |name| name.clone());
            let field_type = &field.ty;

            let write_type = quote! { dioxus_stores::macro_helpers::dioxus_signals::MappedMutSignal<#field_type, __W> };

            let store_type = if foreign {
                quote! { dioxus_stores::ForeignStore<#field_type, #write_type> }
            } else {
                quote! { <#field_type as dioxus_stores::Storable>::Store<
                    #write_type
                > }
            };

            let store_constructor = if foreign {
                quote! { dioxus_stores::ForeignStore::new }
            } else {
                quote! { <#field_type as dioxus_stores::Storable>::create_selector }
            };

            let ordinal = i as u32;

            Ok::<_, syn::Error>(quote! {
                fn #function_name(
                    self,
                ) -> #store_type {
                    let __map_field: fn(&#struct_name #ty_generics) -> &#field_type = |value| &value.#field_accessor;
                    let __map_mut_field: fn(&mut #struct_name #ty_generics) -> &mut #field_type = |value| &mut value.#field_accessor;
                    let scope = self.selector.scope(
                        #ordinal,
                        __map_field,
                        __map_mut_field,
                    );
                    #store_constructor(scope)
                }
            })
        })
        .collect::<syn::Result<Vec<_>>>()?;

    // Generate the store implementation
    let expanded = quote! {
        impl #impl_generics dioxus_stores::Storable for #struct_name #ty_generics #where_clause {
            type Store<__W: dioxus_stores::macro_helpers::dioxus_signals::Writable<Target = Self>> = #selector_name #selector_ty_generics;

            fn create_selector<__W: dioxus_stores::macro_helpers::dioxus_signals::Writable<Target = Self>>(selector: dioxus_stores::SelectorScope<__W>) -> Self::Store<__W> {
                #selector_name { selector, _phantom: std::marker::PhantomData }
            }
        }

        struct #selector_name #selector_generics #selector_where_clause {
            selector: dioxus_stores::SelectorScope<__W>,
            _phantom: std::marker::PhantomData<#struct_name #ty_generics>,
        }

        impl #selector_impl_generics std::clone::Clone for #selector_name #selector_ty_generics #selector_clone_where_clause {
            fn clone(&self) -> Self {
                Self {
                    selector: self.selector.clone(),
                    _phantom: std::marker::PhantomData,
                }
            }
        }

        impl #selector_impl_generics std::marker::Copy for #selector_name #selector_ty_generics #selector_copy_where_clause {}

        impl #selector_impl_generics std::cmp::PartialEq for #selector_name #selector_ty_generics #selector_partial_eq_where_clause {
            fn eq(&self, other: &Self) -> bool {
                self.selector == other.selector
            }
        }

        impl #selector_impl_generics #selector_name #selector_ty_generics #selector_map_where_clause {
            #(
                #fields
            )*
        }

        #store_struct_into_boxed

        #store_struct_readable

        #store_struct_writable
    };

    Ok(expanded)
}

fn derive_store_struct_into_boxed(
    input: &DeriveInput,
    selector_name: &Ident,
) -> syn::Result<TokenStream2> {
    let struct_name = &input.ident;

    let (_, ty_generics, _) = input.generics.split_for_impl();
    let mut impl_generics = input.generics.clone();
    impl_generics
        .params
        .push(parse_quote!(__W: Writable<Storage = UnsyncStorage> + 'static));
    impl_generics
        .params
        .push(parse_quote!(__F: Fn(&__W::Target) -> &#struct_name #ty_generics + 'static));
    impl_generics.params.push(
        parse_quote!(__FMut: Fn(&mut __W::Target) -> &mut #struct_name #ty_generics + 'static),
    );
    let (impl_generics, _, where_clause) = impl_generics.split_for_impl();

    let general_selector_ty_generics: Option<AngleBracketedGenericArguments> =
        syn::parse2(ty_generics.to_token_stream()).ok();
    let extra = parse_quote!(dioxus_stores::macro_helpers::dioxus_signals::MappedMutSignal<#struct_name #ty_generics, __W, __F, __FMut>);
    let general_selector_ty_generics = match general_selector_ty_generics {
        Some(mut args) => {
            args.args.push(extra);
            args
        }
        None => parse_quote! {<#extra>},
    };

    let boxed_selector_ty_generics: Option<AngleBracketedGenericArguments> =
        syn::parse2(ty_generics.to_token_stream()).ok();
    let extra = parse_quote!(dioxus_stores::macro_helpers::dioxus_signals::WriteSignal<#struct_name #ty_generics>);
    let boxed_selector_ty_generics = match boxed_selector_ty_generics {
        Some(mut args) => {
            args.args.push(extra);
            args
        }
        None => parse_quote! {<#extra>},
    };

    Ok(quote! {
        impl #impl_generics ::std::convert::From<#selector_name #general_selector_ty_generics>
            for #selector_name #boxed_selector_ty_generics
            #where_clause
        {
            fn from(value: #selector_name #general_selector_ty_generics) -> Self {
                #selector_name {
                    selector: value.selector.map(::std::convert::Into::into),
                    _phantom: std::marker::PhantomData,
                }
            }
        }
    })
}

fn derive_store_struct_readable(
    input: &DeriveInput,
    selector_name: &Ident,
) -> syn::Result<TokenStream2> {
    let struct_name = &input.ident;

    let (_, original_ty_generics, _) = input.generics.split_for_impl();
    let mut generics = input.generics.clone();
    generics
        .params
        .push(parse_quote!(__W: dioxus_stores::macro_helpers::dioxus_signals::Readable<Target = #struct_name #original_ty_generics>));

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    Ok(quote! {
        impl #impl_generics dioxus_stores::macro_helpers::dioxus_signals::Readable
            for #selector_name #ty_generics
            #where_clause
        {
            type Storage = __W::Storage;
            type Target = #struct_name #original_ty_generics;

            fn try_read_unchecked(&self) -> Result<dioxus_stores::macro_helpers::dioxus_signals::ReadableRef<'static, Self>, dioxus_stores::macro_helpers::dioxus_signals::BorrowError> {
                self.selector.try_read_unchecked()
            }

            fn try_peek_unchecked(&self) -> Result<dioxus_stores::macro_helpers::dioxus_signals::ReadableRef<'static, Self>, dioxus_stores::macro_helpers::dioxus_signals::BorrowError> {
                self.selector.try_peek_unchecked()
            }

            fn subscribers(&self) -> Option<dioxus_stores::macro_helpers::dioxus_core::Subscribers> {
                self.selector.subscribers()
            }
        }
    })
}

fn derive_store_struct_writable(
    input: &DeriveInput,
    selector_name: &Ident,
) -> syn::Result<TokenStream2> {
    let struct_name = &input.ident;

    let (_, original_ty_generics, _) = input.generics.split_for_impl();
    let mut generics = input.generics.clone();
    generics
        .params
        .push(parse_quote!(__W: dioxus_stores::macro_helpers::dioxus_signals::Writable<Target = #struct_name #original_ty_generics>));
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    Ok(quote! {
        impl #impl_generics dioxus_stores::macro_helpers::dioxus_signals::Writable
            for #selector_name #ty_generics
            #where_clause
        {
            type WriteMetadata = <__W as dioxus_stores::macro_helpers::dioxus_signals::Writable>::WriteMetadata;

            fn try_write_unchecked(&self) -> Result<dioxus_stores::macro_helpers::dioxus_signals::WritableRef<'static, Self>, dioxus_stores::macro_helpers::dioxus_signals::BorrowMutError> {
                self.selector.try_write_unchecked()
            }
        }
    })
}

enum StoreAttribute {
    Foreign,
}

impl StoreAttribute {
    fn from_attribute(attr: &syn::Attribute) -> Option<syn::Result<Self>> {
        attr.path().is_ident("store").then(|| attr.parse_args())
    }
}

impl Parse for StoreAttribute {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let ident: syn::Ident = input.parse()?;
        match ident.to_string().as_str() {
            "foreign" => Ok(StoreAttribute::Foreign),
            _ => Err(syn::Error::new(ident.span(), "Unknown store attribute")),
        }
    }
}
