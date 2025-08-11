use proc_macro2::TokenStream;
use quote::quote;
use syn::parse::Parse;
use syn::spanned::Spanned;
use syn::{
    parse_quote, Ident, ImplItem, ImplItemConst, ImplItemType, ItemImpl, PathArguments, Type,
    WherePredicate,
};

pub(crate) fn extend_store(args: ExtendArgs, mut input: ItemImpl) -> syn::Result<TokenStream> {
    // Extract the type name the store is generic over
    let store_type = &*input.self_ty;
    let store = parse_store_type(store_type)?;
    let store_path = &store.store_path;
    let item = store.store_generic;
    let lens_generic = store.store_lens;
    let visibility = args
        .visibility
        .unwrap_or_else(|| syn::Visibility::Inherited);
    if input.trait_.is_some() {
        return Err(syn::Error::new_spanned(
            input.trait_.unwrap().1,
            "The `store` attribute can only be used on `impl Store<T> { ... }` blocks, not trait implementations.",
        ));
    }

    let extension_name = match args.name {
        Some(attr) => attr,
        None => {
            // Otherwise, generate a name based on the type name
            let type_name = stringify_type(&item)?;
            Ident::new(&format!("{}StoreImplExt", type_name), item.span())
        }
    };

    // Go through each method in the impl block and add extra bounds to lens as needed
    let immutable_bounds: WherePredicate = parse_quote!(#lens_generic: dioxus_stores::macro_helpers::dioxus_signals::Readable<Target = #item> + ::std::marker::Copy + 'static);
    let mutable_bounds: WherePredicate = parse_quote!(#lens_generic: dioxus_stores::macro_helpers::dioxus_signals::Writable<Target = #item> + ::std::marker::Copy + 'static);
    for item in &mut input.items {
        let ImplItem::Fn(func) = item else {
            continue; // Only process function items
        };
        // Only add bounds if the function has a self argument
        let Some(receiver) = func.sig.inputs.iter().find_map(|arg| {
            if let syn::FnArg::Receiver(receiver) = arg {
                Some(receiver)
            } else {
                None
            }
        }) else {
            continue;
        };
        let extra_bounds = match (&receiver.reference, &receiver.mutability) {
            // The function takes &self
            (Some(_), None) => &immutable_bounds,
            // The function takes &mut self
            (Some(_), Some(_)) => &mutable_bounds,
            _ => {
                // If the function doesn't take &self or &mut self, we don't need to add any bounds
                continue;
            }
        };
        func.sig
            .generics
            .make_where_clause()
            .predicates
            .push(extra_bounds.clone());
    }

    // Push a __Lens generic to the impl if it doesn't already exist
    let contains_lens_generic = input.generics.params.iter().any(|param| {
        if let syn::GenericParam::Type(ty) = param {
            ty.ident == lens_generic
        } else {
            false
        }
    });
    if !contains_lens_generic {
        input
            .generics
            .params
            .push(parse_quote!(#lens_generic: ::std::marker::Copy + 'static));
    }

    // quote as the trait definition
    let trait_definition = impl_to_trait_body(&extension_name, &input, &visibility)?;

    // Reformat the type to be generic over the lens
    input.self_ty = Box::new(parse_quote!(#store_path<#item, #lens_generic>));
    // Change the standalone impl block to a trait impl block
    let (_, trait_generics, _) = input.generics.split_for_impl();
    input.trait_ = Some((
        None,
        parse_quote!(#extension_name #trait_generics),
        parse_quote!(for),
    ));

    Ok(quote! {
        #trait_definition

        #input
    })
}

fn stringify_type(ty: &Type) -> syn::Result<String> {
    match ty {
        Type::Array(type_array) => {
            let elem = stringify_type(&type_array.elem)?;
            Ok(format!("Array{elem}"))
        }
        Type::Slice(type_slice) => {
            let elem = stringify_type(&type_slice.elem)?;
            Ok(format!("Slice{elem}"))
        }
        Type::Paren(type_paren) => stringify_type(&type_paren.elem),
        Type::Path(type_path) => {
            let last_segment = type_path.path.segments.last().ok_or_else(|| {
                syn::Error::new_spanned(type_path, "Type path must have at least one segment")
            })?;
            let ident = &last_segment.ident;
            Ok(ident.to_string())
        }
        _ => Err(syn::Error::new_spanned(
            ty,
            "Unsupported type in store implementation",
        )),
    }
}

fn impl_to_trait_body(
    trait_name: &Ident,
    item: &ItemImpl,
    visibility: &syn::Visibility,
) -> syn::Result<TokenStream> {
    let ItemImpl {
        attrs,
        defaultness,
        unsafety,
        items,
        ..
    } = item;

    let generics = &item.generics;

    let items = items
        .iter()
        .map(item_to_trait_definition)
        .collect::<syn::Result<Vec<_>>>()?;

    Ok(quote! {
        #(#attrs)*
        #visibility #defaultness #unsafety trait #trait_name #generics {
            #(#items)*
        }
    })
}

fn item_to_trait_definition(item: &syn::ImplItem) -> syn::Result<proc_macro2::TokenStream> {
    match item {
        syn::ImplItem::Fn(func) => {
            let sig = &func.sig;

            Ok(quote! {
                #sig;
            })
        }
        syn::ImplItem::Const(impl_item_const) => {
            let ImplItemConst {
                attrs,
                const_token,
                ident,
                generics,
                colon_token,
                ty,
                semi_token,
                ..
            } = impl_item_const;

            Ok(quote! {
                #(#attrs)*
                #const_token #ident #generics #colon_token #ty #semi_token
            })
        }
        syn::ImplItem::Type(impl_item_type) => {
            let ImplItemType {
                attrs,
                type_token,
                ident,
                generics,
                eq_token,
                ty,
                semi_token,
                ..
            } = impl_item_type;

            Ok(quote! {
                #(#attrs)*
                #type_token #ident #generics #eq_token #ty #semi_token
            })
        }
        _ => Err(syn::Error::new_spanned(item, "Unsupported item type")),
    }
}

fn argument_as_type(arg: &syn::GenericArgument) -> Option<Type> {
    if let syn::GenericArgument::Type(ty) = arg {
        Some(ty.clone())
    } else {
        None
    }
}

struct StorePath {
    store_path: syn::Path,
    store_generic: syn::Type,
    store_lens: syn::Ident,
}

fn parse_store_type(store_type: &Type) -> syn::Result<StorePath> {
    if let Type::Path(type_path) = store_type {
        if let Some(segment) = type_path.path.segments.last() {
            if let PathArguments::AngleBracketed(args) = &segment.arguments {
                if let Some(store_generics) = args.args.first().and_then(argument_as_type) {
                    let store_lens = args
                        .args
                        .iter()
                        .nth(1)
                        .and_then(argument_as_type)
                        .unwrap_or_else(|| parse_quote!(__Lens));
                    let store_lens = parse_quote!(#store_lens);
                    let mut path_without_generics = type_path.path.clone();
                    for segment in &mut path_without_generics.segments {
                        segment.arguments = PathArguments::None;
                    }
                    return Ok(StorePath {
                        store_path: path_without_generics,
                        store_generic: store_generics,
                        store_lens,
                    });
                }
            }
        }
    }
    Err(syn::Error::new_spanned(
        store_type,
        "The implementation must be in the form `impl Store<T> {...}`",
    ))
}

/// The args the `#[store]` attribute macro accepts
pub(crate) struct ExtendArgs {
    /// The name of the extension trait generated
    name: Option<Ident>,
    /// The visibility of the extension trait
    visibility: Option<syn::Visibility>,
}

impl Parse for ExtendArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        // Try to parse visibility if it exists
        let visibility = if input.peek(syn::Token![pub]) {
            let vis: syn::Visibility = input.parse()?;
            Some(vis)
        } else {
            None
        };
        // Try to parse name = ident if it exists
        let name = if input.peek(Ident) && input.peek2(syn::Token![=]) {
            let ident: Ident = input.parse()?;
            if ident != "name" {
                return Err(syn::Error::new_spanned(ident, "Expected `name` argument"));
            }
            let _eq_token: syn::Token![=] = input.parse()?;
            let ident: Ident = input.parse()?;
            Some(ident)
        } else {
            None
        };
        Ok(ExtendArgs { name, visibility })
    }
}
