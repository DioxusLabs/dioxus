use proc_macro2::TokenStream;
use quote::quote;
use syn::parse::Parse;
use syn::spanned::Spanned;
use syn::{
    parse_quote, Generics, Ident, ImplItem, ImplItemConst, ImplItemType, ItemImpl, PathArguments,
    Type, Visibility, WherePredicate,
};

pub(crate) fn extend_store(args: ExtendArgs, mut input: ItemImpl) -> syn::Result<TokenStream> {
    // Extract the type name the store is generic over
    let store_type = &*input.self_ty;
    let store = parse_store_type(store_type)?;
    let store_path = &store.store_path;
    let item = store.store_generic;
    let lens_generic = store.store_lens;
    if let Some(input_trait) = input.trait_.as_ref() {
        return Err(syn::Error::new_spanned(
            input_trait.1.clone(),
            "The `store` attribute can only be used on `impl Store<T> { ... }` blocks, not trait implementations.",
        ));
    }

    let type_name = stringify_type(&item)?;
    let extension_name = match args.name {
        Some(attr) => attr,
        None => Ident::new(&format!("{}StoreImplExt", type_name), item.span()),
    };
    let name_prefix = format!("{}StoreImpl", type_name);

    // Go through each method in the impl block and add extra bounds to lens as needed
    let immutable_bounds: WherePredicate = parse_quote!(#lens_generic: dioxus_stores::macro_helpers::dioxus_signals::Readable<Target = #item> + ::std::marker::Copy + 'static);
    let mutable_bounds: WherePredicate = parse_quote!(#lens_generic: dioxus_stores::macro_helpers::dioxus_signals::Writable<Target = #item> + ::std::marker::Copy + 'static);
    for impl_item in &mut input.items {
        let ImplItem::Fn(func) = impl_item else {
            continue;
        };
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
            (Some(_), None) => &immutable_bounds,
            (Some(_), Some(_)) => &mutable_bounds,
            _ => continue,
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

    // Bucket each method by its declared visibility. Bucket 0 is always `pub`
    // so any method declared `pub` is callable wherever the trait is in scope.
    // Other visibilities get their own bucket, their own marker type (with the
    // method's visibility) and their own private witness trait. Calling the
    // method outside that visibility fails to infer the witness parameter.
    let pub_vis: Visibility = parse_quote!(pub);
    let mut visibility_order: Vec<Visibility> = vec![pub_vis];
    let mut method_buckets: Vec<usize> = Vec::new();
    for impl_item in &input.items {
        if let ImplItem::Fn(func) = impl_item {
            let idx = visibility_order
                .iter()
                .position(|v| v == &func.vis)
                .unwrap_or_else(|| {
                    let i = visibility_order.len();
                    visibility_order.push(func.vis.clone());
                    i
                });
            method_buckets.push(idx);
        }
    }

    let marker_idents: Vec<Ident> = visibility_order
        .iter()
        .map(|v| {
            Ident::new(
                &format!(
                    "__{}Marker{}",
                    name_prefix,
                    crate::derive::visibility_suffix(v)
                ),
                item.span(),
            )
        })
        .collect();
    let witness_idents: Vec<Ident> = visibility_order
        .iter()
        .map(|v| {
            Ident::new(
                &format!(
                    "__{}VisibleIn{}",
                    name_prefix,
                    crate::derive::visibility_suffix(v)
                ),
                item.span(),
            )
        })
        .collect();

    let marker_decls: Vec<TokenStream> = visibility_order
        .iter()
        .zip(&marker_idents)
        .map(|(vis, name)| {
            quote! {
                #[doc(hidden)]
                #[allow(non_camel_case_types)]
                #vis struct #name;
            }
        })
        .collect();
    let witness_trait_decls: Vec<TokenStream> = witness_idents
        .iter()
        .map(|name| {
            quote! {
                #[doc(hidden)]
                #[allow(non_camel_case_types)]
                trait #name<__V> {}
            }
        })
        .collect();

    // Witness + sealed impls are blanket over the impl's generics (which now
    // include __Lens) but not over __V.
    let (impl_generics, _, where_clause) = input.generics.split_for_impl();
    let store_ty = quote! { #store_path<#item, #lens_generic> };
    let witness_impls: Vec<TokenStream> = witness_idents
        .iter()
        .zip(&marker_idents)
        .map(|(trait_name, marker)| {
            quote! {
                impl #impl_generics #trait_name<#marker> for #store_ty #where_clause {}
            }
        })
        .collect();

    // Sealed supertrait prevents external impls of the extension trait itself.
    let sealed_name = Ident::new(&format!("__{}Sealed", name_prefix), item.span());
    let sealed_impl = quote! {
        #[doc(hidden)]
        #[allow(non_camel_case_types)]
        trait #sealed_name {}
        impl #impl_generics #sealed_name for #store_ty #where_clause {}
    };

    // Gate each method on `Self: __WitnessN<__V>` so the caller must name a
    // marker in scope. Strip the per-method visibility since trait items don't
    // take one; the seal does the job instead.
    let mut fn_cursor = 0usize;
    for impl_item in &mut input.items {
        if let ImplItem::Fn(func) = impl_item {
            let bucket = method_buckets[fn_cursor];
            fn_cursor += 1;
            let witness = &witness_idents[bucket];
            let bound: WherePredicate = parse_quote!(Self: #witness<__V>);
            func.sig.generics.make_where_clause().predicates.push(bound);
            func.vis = Visibility::Inherited;
        }
    }

    // Extension trait carries __V as its first generic so method bodies can
    // bind `Self: __WitnessN<__V>` against a caller-inferred marker.
    let mut extension_generics = input.generics.clone();
    extension_generics.params.insert(0, parse_quote!(__V));

    let trait_definition =
        impl_to_trait_body(&extension_name, &sealed_name, &input, &extension_generics)?;

    // Reformat the type and generics for the trait impl block
    input.self_ty = parse_quote!(#store_ty);
    input.generics = extension_generics;
    let (_, trait_generics, _) = input.generics.split_for_impl();
    input.trait_ = Some((
        None,
        parse_quote!(#extension_name #trait_generics),
        parse_quote!(for),
    ));

    Ok(quote! {
        #(#marker_decls)*
        #(#witness_trait_decls)*
        #(#witness_impls)*
        #sealed_impl

        #trait_definition

        #[allow(private_bounds)]
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
    sealed_name: &Ident,
    item: &ItemImpl,
    extension_generics: &Generics,
) -> syn::Result<TokenStream> {
    let ItemImpl {
        attrs,
        defaultness,
        unsafety,
        items,
        ..
    } = item;

    let (ext_impl_generics, _, ext_where) = extension_generics.split_for_impl();

    let items = items
        .iter()
        .map(item_to_trait_definition)
        .collect::<syn::Result<Vec<_>>>()?;

    Ok(quote! {
        #(#attrs)*
        #[allow(private_bounds)]
        pub #defaultness #unsafety trait #trait_name #ext_impl_generics: #sealed_name #ext_where {
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
}

impl Parse for ExtendArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
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
        Ok(ExtendArgs { name })
    }
}
