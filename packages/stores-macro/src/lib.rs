use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote, ToTokens};
use syn::{parse::Parse, parse_macro_input, spanned::Spanned, DataStruct, DeriveInput, Index};

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
                impl dioxus_stores::macro_helpers::dioxus_signals::Writable<Target = #field_type, Storage = S> + Copy + 'static,
                S,
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
        impl dioxus_stores::Selectable for #struct_name {
            type Selector<View, S: dioxus_stores::SelectorStorage> = #selector_name<View, S>;
        }

        struct #selector_name<W, S: dioxus_stores::SelectorStorage = dioxus_stores::macro_helpers::dioxus_signals::UnsyncStorage> {
            selector: dioxus_stores::SelectorScope<W, S>,
        }

        impl<W, S: dioxus_stores::SelectorStorage> dioxus_stores::CreateSelector for #selector_name<W, S> {
            type View = W;
            type Storage = S;

            fn new(selector: dioxus_stores::SelectorScope<Self::View, Self::Storage>) -> Self {
                Self { selector }
            }
        }

        impl<W: dioxus_stores::macro_helpers::dioxus_signals::Writable<Target = Value, Storage = S> + Copy + 'static, S: dioxus_stores::SelectorStorage>
            #selector_name<W, S>
        {
            #(
                #fields
            )*
        }
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
