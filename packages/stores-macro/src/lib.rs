use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote, ToTokens};
use syn::{
    parse::Parse, parse_macro_input, parse_quote, punctuated::Punctuated, spanned::Spanned,
    DataStruct, DeriveInput, Index,
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
        syn::Data::Enum(data_enum) => {
            return Err(syn::Error::new(
                input.span(),
                "Store macro does not support enums",
            ))
        }
        syn::Data::Union(data_union) => {
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
    selector_generics
        .params
        .push(parse_quote!(__S: dioxus_stores::SelectorStorage = dioxus_stores::macro_helpers::dioxus_signals::UnsyncStorage));

    let (selector_impl_generics, selector_ty_generics, selector_where_clause) =
        selector_generics.split_for_impl();

    let mut selector_map_bounds: Punctuated<syn::WherePredicate, syn::Token![,]> =
        Punctuated::new();
    selector_map_bounds.push(
        parse_quote!(__W: dioxus_stores::macro_helpers::dioxus_signals::Writable<Target = #struct_name #ty_generics, Storage = __S> + Copy + 'static),
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

    let mut selector_clone_bounds: Punctuated<syn::WherePredicate, syn::Token![+]> =
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

    let mut selector_partial_eq_bounds: Punctuated<syn::WherePredicate, syn::Token![+]> =
        Punctuated::new();
    selector_partial_eq_bounds.push(parse_quote!(__W: ::std::cmp::PartialEq));
    let selector_partial_eq_where_clause = if let Some(mut clause) = generics.where_clause.clone() {
        clause.predicates.extend(selector_partial_eq_bounds);
        clause.into_token_stream()
    } else {
        quote! { where #selector_partial_eq_bounds }
    };

    let fields = fields.iter().enumerate().map(|(i, field)| {
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
        let function_name = field_name.as_ref().map_or_else(
            || format_ident!("field_{i}"),
            |name| name.clone(),
        );
        let field_type = &field.ty;

        let foreign_type = if foreign {
            quote! { dioxus_stores::ForeignType<#field_type> }
        } else {
            quote! { #field_type }
        };

        let ordinal = i as u32;

        Ok::<_, syn::Error>(quote! {
            fn #function_name(
                self,
            ) -> <#foreign_type as dioxus_stores::Selectable>::Selector<
                // impl dioxus_stores::macro_helpers::dioxus_signals::Writable<Target = #field_type, Storage = __S> + Copy + 'static,
                dioxus_stores::macro_helpers::dioxus_signals::MappedMutSignal<#field_type, __W, impl Fn(&#struct_name #ty_generics) -> &#field_type + Copy + 'static, impl Fn(&mut #struct_name #ty_generics) -> &mut #field_type + Copy + 'static>,
                __S,
            > {
                dioxus_stores::CreateSelector::new(self.selector.scope(
                    #ordinal,
                    |value| &value.#field_accessor,
                    |value| &mut value.#field_accessor,
                ))
            }
        })
    }).collect::<syn::Result<Vec<_>>>()?;

    // Generate the store implementation
    let expanded = quote! {
        impl #impl_generics dioxus_stores::Selectable for #struct_name #ty_generics #where_clause {
            type Selector<__W, __S: dioxus_stores::SelectorStorage> = #selector_name #selector_ty_generics;
        }

        struct #selector_name #selector_generics #selector_where_clause {
            selector: dioxus_stores::SelectorScope<__W, __S>,
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

        impl #selector_impl_generics dioxus_stores::CreateSelector for #selector_name #selector_ty_generics #selector_where_clause {
            type View = __W;
            type Storage = __S;

            fn new(selector: dioxus_stores::SelectorScope<Self::View, Self::Storage>) -> Self {
                Self { selector, _phantom: std::marker::PhantomData }
            }
        }

        impl #selector_impl_generics #selector_name #selector_ty_generics #selector_map_where_clause {
            #(
                #fields
            )*
        }

        // impl #selector_impl_generics From<#selector_name #selector_ty_generics> for #selector_name #selector_ty_generics {
        //     fn from(value: #selector_name #selector_ty_generics) -> Self {
        //         Self {
        //             selector: value.selector.map(|w| w.into()),
        //             _phantom: std::marker::PhantomData
        //         }
        //     }
        // }
    };

    Ok(expanded)
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
