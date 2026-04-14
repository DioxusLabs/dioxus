use proc_macro2::TokenStream;
use quote::quote;
use syn::parse::Parse;
use syn::spanned::Spanned;
use syn::{
    parse_quote, Ident, ImplItem, ImplItemConst, ImplItemType, ItemImpl, PathArguments, Type,
    WherePredicate,
};

pub(crate) fn extend_store(args: ExtendArgs, mut input: ItemImpl) -> syn::Result<TokenStream> {
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

    // Push a __Lens generic to the impl if it doesn't already exist.
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

    let store_ty = quote! { #store_path<#item, #lens_generic> };

    // Seal generics = the impl's generics (with __Lens). Trait generics add
    // __V at the front so each method can be gated on `Self: __Witness<__V>`.
    let mut extension_generics = input.generics.clone();
    extension_generics.params.insert(0, parse_quote!(__V));
    let (seal_impl, _, seal_where) = input.generics.split_for_impl();
    let (trait_decl, trait_use, trait_where) = extension_generics.split_for_impl();

    let mut seal =
        crate::seal::SealBuilder::new(name_prefix, item.span(), store_ty.clone(), extension_name)
            .seal_generics(quote! { #seal_impl }, quote! { #seal_where })
            .trait_generics(
                quote! { #trait_decl },
                quote! { #trait_use },
                quote! { #trait_where },
            )
            .trait_visibility(args.visibility.unwrap_or_else(|| parse_quote!(pub)));

    let immutable_bounds: WherePredicate = parse_quote!(#lens_generic: dioxus_stores::macro_helpers::dioxus_signals::Readable<Target = #item> + ::std::marker::Copy + 'static);
    let mutable_bounds: WherePredicate = parse_quote!(#lens_generic: dioxus_stores::macro_helpers::dioxus_signals::Writable<Target = #item> + ::std::marker::Copy + 'static);

    for impl_item in input.items {
        match impl_item {
            ImplItem::Fn(mut func) => {
                // Every fn — receiver or not — is gated on a witness for its
                // declared visibility so static methods honor the same seal as
                // `&self` / `&mut self` methods. Additionally, `&self` adds a
                // `Readable` bound on __Lens and `&mut self` adds `Writable`.
                let receiver = func.sig.inputs.iter().find_map(|arg| {
                    if let syn::FnArg::Receiver(r) = arg {
                        Some(r)
                    } else {
                        None
                    }
                });
                let extra = receiver.and_then(|r| match (&r.reference, &r.mutability) {
                    (Some(_), None) => Some(&immutable_bounds),
                    (Some(_), Some(_)) => Some(&mutable_bounds),
                    _ => None,
                });
                if let Some(extra) = extra {
                    func.sig
                        .generics
                        .make_where_clause()
                        .predicates
                        .push(extra.clone());
                }
                let witness = seal.push_witness(&func.vis);
                let bound: WherePredicate = parse_quote!(Self: #witness<__V>);
                func.sig.generics.make_where_clause().predicates.push(bound);
                let sig = &func.sig;
                let body = &func.block;
                seal.push_method(quote! { #sig }, quote! { #body });
            }
            ImplItem::Const(c) => {
                let ImplItemConst {
                    attrs,
                    const_token,
                    ident,
                    generics,
                    colon_token,
                    ty,
                    semi_token,
                    ..
                } = &c;
                let trait_item = quote! {
                    #(#attrs)*
                    #const_token #ident #generics #colon_token #ty #semi_token
                };
                let impl_item = quote! { #c };
                seal.push_assoc(trait_item, impl_item);
            }
            ImplItem::Type(t) => {
                let ImplItemType {
                    attrs,
                    type_token,
                    ident,
                    generics,
                    eq_token,
                    ty,
                    semi_token,
                    ..
                } = &t;
                let trait_item = quote! {
                    #(#attrs)*
                    #type_token #ident #generics #eq_token #ty #semi_token
                };
                let impl_item = quote! { #t };
                seal.push_assoc(trait_item, impl_item);
            }
            other => return Err(syn::Error::new_spanned(other, "Unsupported item type")),
        }
    }

    Ok(seal.into_tokens())
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
    /// The visibility of the extension trait itself. Defaults to `pub`.
    visibility: Option<syn::Visibility>,
}

impl Parse for ExtendArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        // An optional leading visibility, e.g. `#[store(pub(crate))]` or
        // `#[store(pub(crate), name = Foo)]`.
        let visibility = if input.peek(syn::Token![pub]) {
            let vis: syn::Visibility = input.parse()?;
            let _: Option<syn::Token![,]> = input.parse()?;
            Some(vis)
        } else {
            None
        };
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
