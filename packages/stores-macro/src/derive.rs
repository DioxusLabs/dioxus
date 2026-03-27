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

/// A field is "more private" than the struct when the struct has an explicit
/// visibility (e.g. `pub`) but the field uses inherited (no modifier) visibility.
fn field_is_more_private(struct_vis: &syn::Visibility, field_vis: &syn::Visibility) -> bool {
    !matches!(struct_vis, syn::Visibility::Inherited)
        && matches!(field_vis, syn::Visibility::Inherited)
}

/// Emit a trait definition + blanket impl for `Store<T, __Lens>`.
fn extension_trait(
    visibility: &syn::Visibility,
    trait_name: &Ident,
    store_ty: &TokenStream2,
    extension_impl_generics: &syn::ImplGenerics,
    extension_ty_generics: &syn::TypeGenerics,
    extension_where_clause: Option<&syn::WhereClause>,
    definitions: &[TokenStream2],
    implementations: &[TokenStream2],
) -> TokenStream2 {
    quote! {
        #visibility trait #trait_name #extension_impl_generics #extension_where_clause {
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
fn can_use_type_alias<'a>(
    generics: &syn::Generics,
    mut fields: impl Iterator<Item = &'a Field>,
) -> bool {
    generics.type_params().all(|p| p.bounds.is_empty())
        && fields.all(|f| field_type_generic(f, generics))
}

fn field_type_generic(field: &Field, generics: &syn::Generics) -> bool {
    generics.type_params().any(|param| {
        matches!(&field.ty, syn::Type::Path(type_path) if type_path.path.is_ident(&param.ident))
    })
}

fn function_name_from_field(index: usize, field: &syn::Field) -> Ident {
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

/// Generate a single field accessor's trait definition, implementation, and transposed store type.
fn generate_field_method(
    field_index: usize,
    field: &syn::Field,
    struct_name: &Ident,
    ty_generics: &syn::TypeGenerics,
) -> (TokenStream2, TokenStream2, TokenStream2) {
    let field_accessor = field.ident.as_ref().map_or_else(
        || Index::from(field_index).to_token_stream(),
        |name| name.to_token_stream(),
    );
    let function_name = function_name_from_field(field_index, field);
    let field_type = &field.ty;
    let store_type = mapped_type(struct_name, ty_generics, field_type);
    let ordinal = LitInt::new(&field_index.to_string(), field.span());

    let definition = quote! {
        fn #function_name(self) -> #store_type;
    };
    let implementation = quote! {
        fn #function_name(self) -> #store_type {
            let __map_field: fn(&#struct_name #ty_generics) -> &#field_type = |value| &value.#field_accessor;
            let __map_mut_field: fn(&mut #struct_name #ty_generics) -> &mut #field_type = |value| &mut value.#field_accessor;
            let scope = self.into_selector().child(#ordinal, __map_field, __map_mut_field);
            ::std::convert::Into::into(scope)
        }
    };

    (store_type, definition, implementation)
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

    // Generate accessor methods for each field, partitioned by visibility.
    let mut public_definitions = Vec::new();
    let mut public_implementations = Vec::new();
    let mut private_definitions = Vec::new();
    let mut private_implementations = Vec::new();
    let mut transposed_fields = Vec::new();

    for (field_index, field) in fields.iter().enumerate() {
        let (transposed, definition, implementation) =
            generate_field_method(field_index, field, struct_name, &ty_generics);
        transposed_fields.push(transposed);
        if field_is_more_private(visibility, &field.vis) {
            private_definitions.push(definition);
            private_implementations.push(implementation);
        } else {
            public_definitions.push(definition);
            public_implementations.push(implementation);
        }
    }

    // Transpose always goes on the public trait. Copy is required because the
    // store is copied into the selector for each field.
    let field_names: Vec<_> = fields
        .iter()
        .enumerate()
        .map(|(i, field)| function_name_from_field(i, field))
        .collect();
    let construct = construct_from_fields(quote! { #transposed_name }, fields, &field_names);
    public_definitions.push(quote! {
        fn transpose(self) -> #transposed_name #extension_ty_generics where Self: ::std::marker::Copy;
    });
    // Each method name is unique to one trait, so `self.method()` resolves
    // unambiguously since both traits are in scope in the generated code.
    public_implementations.push(quote! {
        fn transpose(self) -> #transposed_name #extension_ty_generics where Self: ::std::marker::Copy {
            #( let #field_names = self.#field_names(); )*
            #construct
        }
    });

    let public_trait = extension_trait(
        visibility,
        &extension_trait_name,
        &store_ty,
        &extension_impl_generics,
        &extension_ty_generics,
        extension_where_clause,
        &public_definitions,
        &public_implementations,
    );

    let private_trait = if private_definitions.is_empty() {
        quote! {}
    } else {
        let name = format_ident!("{}PrivateStoreExt", struct_name);
        extension_trait(
            &syn::Visibility::Inherited,
            &name,
            &store_ty,
            &extension_impl_generics,
            &extension_ty_generics,
            extension_where_clause,
            &private_definitions,
            &private_implementations,
        )
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
        #public_trait
        #private_trait
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
    let binding = function_name_from_field(field_index, field);
    let field_type = &field.ty;
    let pattern = if field.ident.is_none() {
        let before = (0..field_index).map(|_| quote!(_));
        let after = (field_index + 1..field_count).map(|_| quote!(_));
        quote!( ( #(#before,)* #binding, #(#after),* ) )
    } else {
        quote!( { #binding, .. })
    };
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
        let scope = self.into_selector().child(#ordinal, __map_field, __map_mut_field);
        ::std::convert::Into::into(scope)
    }
}

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

    let mut definitions = Vec::new();
    let mut implementations = Vec::new();
    let mut transposed_variants = Vec::new();
    let mut transposed_match_arms = Vec::new();

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

            let select_field = select_enum_variant_field(
                enum_name,
                &ty_generics,
                variant_name,
                field,
                i,
                fields.len(),
            );

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

        // Build the match arm that transposes this variant
        let field_names: Vec<_> = fields
            .iter()
            .enumerate()
            .map(|(i, field)| function_name_from_field(i, field))
            .collect();
        let construct_fields = field_names
            .iter()
            .zip(&field_selectors)
            .map(|(name, selector)| quote! { let #name = { #selector }; });
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
            self.selector().track_shallow();
            let read = dioxus_stores::macro_helpers::dioxus_signals::ReadableExt::peek(self.selector());
            match &*read {
                #(#transposed_match_arms)*
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

    let trait_tokens = extension_trait(
        visibility,
        &extension_trait_name,
        &store_ty,
        &extension_impl_generics,
        &extension_ty_generics,
        extension_where_clause,
        &definitions,
        &implementations,
    );

    Ok(quote! {
        #trait_tokens
        #transposed_enum
    })
}
