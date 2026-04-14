use convert_case::{Case, Casing};
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote, ToTokens};
use syn::{
    parse_quote, spanned::Spanned, DataEnum, DataStruct, DeriveInput, Field, Fields, Generics,
    Ident, Index, LitInt, Visibility,
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

/// Emit a trait definition + blanket impl for `Store<T, __Lens>`.
///
/// Used by the enum path, which doesn't have per-variant visibility gating.
/// A private `Sealed` supertrait prevents external types from implementing
/// the extension trait.
#[allow(clippy::too_many_arguments)]
fn extension_trait(
    visibility: &syn::Visibility,
    trait_name: &Ident,
    sealed_trait_name: &Ident,
    store_ty: &TokenStream2,
    extension_impl_generics: &syn::ImplGenerics,
    extension_ty_generics: &syn::TypeGenerics,
    extension_where_clause: Option<&syn::WhereClause>,
    definitions: &[TokenStream2],
    implementations: &[TokenStream2],
) -> TokenStream2 {
    quote! {
        #[doc(hidden)]
        #[allow(non_camel_case_types)]
        trait #sealed_trait_name {}
        impl #extension_impl_generics #sealed_trait_name for #store_ty #extension_where_clause {}

        #visibility trait #trait_name #extension_impl_generics: #sealed_trait_name #extension_where_clause {
            #(#definitions)*
        }

        impl #extension_impl_generics #trait_name #extension_ty_generics for #store_ty #extension_where_clause {
            #(#implementations)*
        }
    }
}

/// Build a constructor expression: `prefix { a, b }`, `prefix(a, b)`, or just `prefix`.
fn construct_from_fields(prefix: TokenStream2, fields: &Fields, names: &[Ident]) -> TokenStream2 {
    match fields {
        Fields::Named(_) => quote! { #prefix { #(#names),* } },
        Fields::Unnamed(_) => quote! { #prefix(#(#names),*) },
        Fields::Unit => quote! { #prefix },
    }
}

/// Zip original fields with their transposed store types, preserving visibility and names.
fn zip_transposed_fields(fields: &Fields, types: &[TokenStream2]) -> Vec<TokenStream2> {
    match fields {
        Fields::Named(f) => f
            .named
            .iter()
            .zip(types)
            .map(|(f, t)| {
                let vis = &f.vis;
                let ident = &f.ident;
                let colon = f.colon_token.as_ref();
                quote! { #vis #ident #colon #t }
            })
            .collect(),
        Fields::Unnamed(f) => f
            .unnamed
            .iter()
            .zip(types)
            .map(|(f, t)| {
                let vis = &f.vis;
                quote! { #vis #t }
            })
            .collect(),
        Fields::Unit => Vec::new(),
    }
}

/// True when a type alias can replace a full transposed struct/enum definition
/// (all type params are unbound and every field is a bare generic type).
///
/// Eg. struct MyStruct<T> {inner: T} can use a type alias to express MyStruct<StoreView<T>>
fn can_use_type_alias<'a>(
    generics: &syn::Generics,
    mut fields: impl Iterator<Item = &'a Field>,
) -> bool {
    generics.type_params().all(|p| p.bounds.is_empty())
        && fields.all(|f| field_type_generic(f, generics))
}

/// Check if a field uses any generics
fn field_type_generic(field: &Field, generics: &syn::Generics) -> bool {
    generics.type_params().any(|param| {
        matches!(&field.ty, syn::Type::Path(type_path) if type_path.path.is_ident(&param.ident))
    })
}

/// Get the function name the extension trait will use to create a selector scoped to a field
fn function_name_from_field(index: usize, field: &syn::Field) -> Ident {
    field
        .ident
        .as_ref()
        .map_or_else(|| format_ident!("field_{index}"), |name| name.clone())
}

/// Get the mapped type for a field
fn mapped_type(
    item: &Ident,
    ty_generics: &syn::TypeGenerics,
    field_type: &syn::Type,
) -> TokenStream2 {
    let lens = quote! {
        dioxus_stores::macro_helpers::dioxus_signals::MappedMutSignal<
            #field_type, __Lens,
            fn(&#item #ty_generics) -> &#field_type,
            fn(&mut #item #ty_generics) -> &mut #field_type,
        >
    };
    quote! { dioxus_stores::Store<#field_type, #lens> }
}

/// Map the original generics into the transposed type's generic arguments.
fn transpose_generics(name: &Ident, generics: &syn::Generics) -> TokenStream2 {
    let (_, ty_generics, _) = generics.split_for_impl();
    let args: Vec<_> = generics
        .params
        .iter()
        .map(|gen| match gen {
            syn::GenericParam::Type(p) => {
                let ident = &p.ident;
                mapped_type(name, &ty_generics, &parse_quote!(#ident))
            }
            syn::GenericParam::Const(p) => {
                let ident = &p.ident;
                quote! { #ident }
            }
            syn::GenericParam::Lifetime(p) => {
                let lt = &p.lifetime;
                quote! { #lt }
            }
        })
        .collect();
    quote!(<#(#args),*>)
}

/// Create the definition for a transposed struct
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
    // If the type only uses generics, we can define this in terms of a type alias on the original type
    // so all of the original type methods remain usable on the transposed version
    if can_use_type_alias(generics, structure.fields.iter()) {
        let generics = transpose_generics(struct_name, generics);
        return quote! { #visibility type #transposed_name #extension_impl_generics = #struct_name #generics; };
    }
    let fields = zip_transposed_fields(&structure.fields, transposed_fields);
    match &structure.fields {
        Fields::Named(_) => quote! {
            #visibility struct #transposed_name #extension_impl_generics #extension_where_clause {
                #(#fields),*
            }
        },
        Fields::Unnamed(_) => quote! {
            #visibility struct #transposed_name #extension_impl_generics(#(#fields),*) #extension_where_clause;
        },
        Fields::Unit => quote! {
            #visibility struct #transposed_name #extension_impl_generics #extension_where_clause;
        },
    }
}

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

    if fields.is_empty() {
        return Ok(quote! {});
    }

    let generics = &input.generics;
    let (_, ty_generics, _) = generics.split_for_impl();
    let (extension_impl_generics, extension_ty_generics, extension_where_clause) =
        extension_generics.split_for_impl();
    let store_ty = quote! { dioxus_stores::Store<#struct_name #ty_generics, __Lens> };

    // Extension-trait generics also carry the visibility witness `__V` as the
    // first type parameter. Methods are gated on `Self: witness_trait<__V>`,
    // which forces the compiler to infer `__V` to a named marker type. Because
    // the marker's visibility matches the field's visibility, the inference
    // silently fails outside the field's scope.
    let mut witness_extension_generics = extension_generics.clone();
    witness_extension_generics
        .params
        .insert(0, parse_quote!(__V));
    let (witness_impl_generics, witness_ty_generics, witness_where_clause) =
        witness_extension_generics.split_for_impl();

    // Bucket fields by visibility. Bucket 0 is reserved for the struct's own
    // visibility so `transpose` can always be called by anyone who can see the
    // type. Rust already guarantees each field's visibility is no wider than
    // the struct's, so using `field.vis` directly is safe and gives the
    // tightest gating the user asked for.
    //
    // The bucket list is tiny (usually ≤ 3 distinct visibilities) so a linear
    // scan with `visibility_eq` is faster and clearer than a hashed lookup.
    let mut visibility_order: Vec<Visibility> = vec![visibility.clone()];
    let mut field_bucket: Vec<usize> = Vec::with_capacity(fields.len());
    for field in fields.iter() {
        let idx = visibility_order
            .iter()
            .position(|v| v == &field.vis)
            .unwrap_or_else(|| {
                let i = visibility_order.len();
                visibility_order.push(field.vis.clone());
                i
            });
        field_bucket.push(idx);
    }
    let transpose_bucket = 0;

    // One marker + one witness trait + one blanket impl per bucket.
    //
    // These are emitted at the same scope as the struct itself (not inside a
    // helper module) so they work equally well for module-level and
    // function-local derives. The marker's visibility is exactly the field's
    // visibility, so `pub(in path)`, `pub(super)`, etc. all propagate
    // faithfully.
    let marker_idents: Vec<Ident> = (0..visibility_order.len())
        .map(|i| format_ident!("__{}StoreMarker{}", struct_name, i))
        .collect();
    let witness_trait_idents: Vec<Ident> = (0..visibility_order.len())
        .map(|i| format_ident!("__{}StoreVisibleAs{}", struct_name, i))
        .collect();

    let marker_decls: Vec<TokenStream2> = visibility_order
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

    let witness_trait_decls: Vec<TokenStream2> = witness_trait_idents
        .iter()
        .map(|name| {
            quote! {
                #[doc(hidden)]
                #[allow(non_camel_case_types)]
                trait #name<__V> {}
            }
        })
        .collect();

    let witness_impls: Vec<TokenStream2> = witness_trait_idents
        .iter()
        .zip(&marker_idents)
        .map(|(trait_name, marker_name)| {
            quote! {
                impl #extension_impl_generics #trait_name<#marker_name>
                    for dioxus_stores::Store<#struct_name #ty_generics, __Lens>
                    #extension_where_clause
                {}
            }
        })
        .collect();

    // Emit each method once as a `(signature, body)` pair so the signature and
    // where-clauses live in a single place.
    let mut methods: Vec<(TokenStream2, TokenStream2)> = Vec::new();
    let mut transposed_fields: Vec<TokenStream2> = Vec::new();
    for (field_index, field) in fields.iter().enumerate() {
        let field_accessor = field.ident.as_ref().map_or_else(
            || Index::from(field_index).to_token_stream(),
            |name| name.to_token_stream(),
        );
        let function_name = function_name_from_field(field_index, field);
        let field_type = &field.ty;
        let store_type = mapped_type(struct_name, &ty_generics, field_type);
        let ordinal = LitInt::new(&field_index.to_string(), field.span());
        transposed_fields.push(store_type.clone());

        let witness_trait = &witness_trait_idents[field_bucket[field_index]];
        let signature = quote! {
            fn #function_name(self) -> #store_type
            where
                Self: #witness_trait<__V>
        };
        let body = quote! {
            {
                let __map_field: fn(&#struct_name #ty_generics) -> &#field_type = |value| &value.#field_accessor;
                let __map_mut_field: fn(&mut #struct_name #ty_generics) -> &mut #field_type = |value| &mut value.#field_accessor;
                let scope = self.into_selector().child(#ordinal, __map_field, __map_mut_field);
                ::std::convert::Into::into(scope)
            }
        };
        methods.push((signature, body));
    }

    // `transpose` is gated on the struct's own visibility bucket.
    let transpose_witness = &witness_trait_idents[transpose_bucket];
    let field_names: Vec<_> = fields
        .iter()
        .enumerate()
        .map(|(i, field)| function_name_from_field(i, field))
        .collect();
    let construct = construct_from_fields(quote! { #transposed_name }, fields, &field_names);
    methods.push((
        quote! {
            fn transpose(self) -> #transposed_name #extension_ty_generics
            where
                Self: ::std::marker::Copy,
                Self: #transpose_witness<__V>
        },
        quote! {
            {
                #( let #field_names = self.#field_names(); )*
                #construct
            }
        },
    ));

    let trait_sigs = methods.iter().map(|(sig, _)| quote! { #sig; });
    let impl_bodies = methods.iter().map(|(sig, body)| quote! { #sig #body });

    let sealed_trait_name = format_ident!("__{}StoreSealed", struct_name);
    let extension_trait_tokens = quote! {
        #[doc(hidden)]
        #[allow(non_camel_case_types)]
        trait #sealed_trait_name {}
        impl #extension_impl_generics #sealed_trait_name for #store_ty #extension_where_clause {}

        #[allow(private_bounds)]
        #visibility trait #extension_trait_name #witness_impl_generics: #sealed_trait_name #witness_where_clause {
            #(#trait_sigs)*
        }

        #[allow(private_bounds)]
        impl #witness_impl_generics #extension_trait_name #witness_ty_generics
            for #store_ty
            #witness_where_clause
        {
            #(#impl_bodies)*
        }
    };

    let transposed_struct = transposed_struct(
        visibility,
        struct_name,
        &transposed_name,
        structure,
        generics,
        &extension_generics,
        &transposed_fields,
    );

    Ok(quote! {
        #(#marker_decls)*
        #(#witness_trait_decls)*
        #(#witness_impls)*
        #extension_trait_tokens
        #transposed_struct
    })
}

/// Generate `is_variant()`: returns `(definition, implementation)`.
fn generate_is_variant_method(
    is_fn: &Ident,
    variant_name: &Ident,
    enum_name: &Ident,
    readable_bounds: &TokenStream2,
) -> (TokenStream2, TokenStream2) {
    let definition = quote! {
        fn #is_fn(&self) -> bool where #readable_bounds;
    };
    let implementation = quote! {
        fn #is_fn(&self) -> bool where #readable_bounds {
            self.selector().track_shallow();
            let ref_self = dioxus_stores::macro_helpers::dioxus_signals::ReadableExt::peek(self.selector());
            matches!(&*ref_self, #enum_name::#variant_name { .. })
        }
    };
    (definition, implementation)
}

/// Generate `variant() -> Option<Store<Field, W>>` for single-field variants.
fn generate_as_variant_method(
    is_fn: &Ident,
    snake_case_variant: &Ident,
    select_field: &TokenStream2,
    store_type: &TokenStream2,
    readable_bounds: &TokenStream2,
) -> (TokenStream2, TokenStream2) {
    let definition = quote! {
        fn #snake_case_variant(self) -> Option<#store_type> where #readable_bounds;
    };
    let implementation = quote! {
        fn #snake_case_variant(self) -> Option<#store_type> where #readable_bounds {
            self.#is_fn().then(|| { #select_field })
        }
    };
    (definition, implementation)
}

fn select_enum_variant_field(
    enum_name: &Ident,
    ty_generics: &syn::TypeGenerics,
    variant_name: &Ident,
    field: &Field,
    field_index: usize,
    field_count: usize,
) -> TokenStream2 {
    // When we map the field, we need to use either the field name for named fields or the index for unnamed fields.
    let binding = function_name_from_field(field_index, field);
    let field_type = &field.ty;
    let pattern = if field.ident.is_none() {
        let before = (0..field_index).map(|_| quote!(_));
        let after = (field_index + 1..field_count).map(|_| quote!(_));
        quote!( ( #(#before,)* #binding, #(#after),* ) )
    } else {
        quote!( { #binding, .. })
    };
    // Each field gets its own reactive scope within the child based on the field's index
    let ordinal = LitInt::new(&field_index.to_string(), variant_name.span());
    quote! {
        let __map_field: fn(&#enum_name #ty_generics) -> &#field_type = |value| match value {
            #enum_name::#variant_name #pattern => #binding,
            _ => panic!("Selector that was created to match {} read after variant changed", stringify!(#variant_name)),
        };
        let __map_mut_field: fn(&mut #enum_name #ty_generics) -> &mut #field_type = |value| match value {
            #enum_name::#variant_name #pattern => #binding,
            _ => panic!("Selector that was created to match {} written after variant changed", stringify!(#variant_name)),
        };
        // Map the field into a child selector that tracks the field
        let scope = self.into_selector().child(#ordinal, __map_field, __map_mut_field);
        // Convert the selector into a store
        ::std::convert::Into::into(scope)
    }
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
    let store_ty = quote! { dioxus_stores::Store<#enum_name #ty_generics, __Lens> };

    // We collect the definitions and implementations for the extension trait methods along with the types of the fields in the transposed enum
    // and the match arms for the transposed enum.
    let mut definitions = Vec::new();
    let mut implementations = Vec::new();
    let mut transposed_variants = Vec::new();
    let mut transposed_match_arms = Vec::new();

    // The generated items that check the variant of the enum need to read the enum which requires these extra bounds
    let readable_bounds = quote! {
        __Lens: dioxus_stores::macro_helpers::dioxus_signals::Readable<Target = #enum_name #ty_generics>,
        #enum_name #ty_generics: 'static
    };

    for variant in variants {
        let variant_name = &variant.ident;
        let snake_case_variant = format_ident!("{}", variant_name.to_string().to_case(Case::Snake));
        let is_fn = format_ident!("is_{}", snake_case_variant);

        let (def, imp) =
            generate_is_variant_method(&is_fn, variant_name, enum_name, &readable_bounds);
        definitions.push(def);
        implementations.push(imp);

        let fields = &variant.fields;
        let mut transposed_fields = Vec::new();
        let mut field_selectors = Vec::new();

        for (i, field) in fields.iter().enumerate() {
            let store_type = mapped_type(enum_name, &ty_generics, &field.ty);
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
                let (def, imp) = generate_as_variant_method(
                    &is_fn,
                    &snake_case_variant,
                    &select_field,
                    &store_type,
                    &readable_bounds,
                );
                definitions.push(def);
                implementations.push(imp);
            }

            field_selectors.push(select_field);
        }

        // Now that we have the types for the field selectors within the variant,
        // we can construct the transposed variant and the logic to turn the normal
        // version of that variant into the store version
        let field_names: Vec<_> = fields
            .iter()
            .enumerate()
            .map(|(i, field)| function_name_from_field(i, field))
            .collect();
        // Turn each field into its store
        let construct_fields = field_names
            .iter()
            .zip(&field_selectors)
            .map(|(name, selector)| quote! { let #name = { #selector }; });
        // Merge the stores into the variant
        let construct_variant = construct_from_fields(
            quote! { #transposed_name::#variant_name },
            fields,
            &field_names,
        );
        transposed_match_arms.push(quote! {
            #enum_name::#variant_name { .. } => {
                #(#construct_fields)*
                #construct_variant
            },
        });

        // Build the transposed variant type definition
        let zipped = zip_transposed_fields(fields, &transposed_fields);
        let transposed_variant = match fields {
            Fields::Named(_) => quote! { #variant_name { #(#zipped),* } },
            Fields::Unnamed(_) => quote! { #variant_name(#(#zipped),*) },
            Fields::Unit => quote! { #variant_name },
        };
        transposed_variants.push(transposed_variant);
    }

    // Transpose method
    definitions.push(quote! {
        fn transpose(self) -> #transposed_name #extension_ty_generics where #readable_bounds, Self: ::std::marker::Copy;
    });
    implementations.push(quote! {
        fn transpose(self) -> #transposed_name #extension_ty_generics where #readable_bounds, Self: ::std::marker::Copy {
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
    });

    // Transposed enum definition
    let all_fields = structure.variants.iter().flat_map(|v| v.fields.iter());
    let transposed_enum = if can_use_type_alias(generics, all_fields) {
        let generics = transpose_generics(enum_name, generics);
        quote! { #visibility type #transposed_name #extension_generics = #enum_name #generics; }
    } else {
        quote! { #visibility enum #transposed_name #extension_impl_generics #extension_where_clause { #(#transposed_variants),* } }
    };

    let sealed_trait_name = format_ident!("__{}StoreSealed", enum_name);
    let trait_tokens = extension_trait(
        visibility,
        &extension_trait_name,
        &sealed_trait_name,
        &store_ty,
        &extension_impl_generics,
        &extension_ty_generics,
        extension_where_clause,
        &definitions,
        &implementations,
    );

    // Expand to the extension trait and its implementation for the store alongside the transposed enum
    Ok(quote! {
        #trait_tokens
        #transposed_enum
    })
}
