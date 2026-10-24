//! Shared builder used by `#[derive(Store)]` and the `#[store]` attribute
//! macro to emit an extension trait + its impl, both gated by a private
//! sealed supertrait and optional per-visibility witness seals.
//!
//! The builder stores only source data (bucket entries, queued items) — all
//! token rendering happens once, inside [`into_tokens`].
//!
//! [`into_tokens`]: SealBuilder::into_tokens

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{Ident, Visibility};

use crate::derive::visibility_suffix;

struct Bucket {
    vis: Visibility,
    marker: Ident,
    witness: Ident,
}

pub(crate) struct SealBuilder {
    prefix: String,
    span: Span,
    store_ty: TokenStream,
    seal_generics: TokenStream,
    seal_where: TokenStream,
    trait_visibility: Visibility,
    trait_name: Ident,
    trait_generics_decl: TokenStream,
    trait_generics_use: TokenStream,
    trait_where: TokenStream,
    buckets: Vec<Bucket>,
    items: Vec<(TokenStream, TokenStream)>,
}

impl SealBuilder {
    /// Start a builder with the minimum set of required fields. Everything
    /// else defaults to empty; configure via the chainable `*_generics`,
    /// `*_where`, and `trait_visibility` setters before `into_tokens`.
    ///
    /// - `prefix`: ident prefix for every generated type, e.g. `"TodoItemStore"`
    ///   → `__TodoItemStoreSealed`, `__TodoItemStoreMarkerPub`,
    ///   `__TodoItemStoreVisibleInPub`.
    /// - `span`: span attached to every generated ident.
    /// - `store_ty`: the store type the extension trait is implemented for,
    ///   e.g. `dioxus_stores::Store<TodoItem, __Lens>`.
    /// - `trait_name`: ident of the extension trait to generate.
    pub(crate) fn new(
        prefix: String,
        span: Span,
        store_ty: TokenStream,
        trait_name: Ident,
    ) -> Self {
        Self {
            prefix,
            span,
            store_ty,
            seal_generics: TokenStream::new(),
            seal_where: TokenStream::new(),
            trait_visibility: Visibility::Inherited,
            trait_name,
            trait_generics_decl: TokenStream::new(),
            trait_generics_use: TokenStream::new(),
            trait_where: TokenStream::new(),
            buckets: Vec::new(),
            items: Vec::new(),
        }
    }

    /// Impl generics + where-clause for the marker / witness / sealed blanket
    /// impls. These do not include `__V`, which lives only on the extension
    /// trait.
    pub(crate) fn seal_generics(mut self, generics: TokenStream, where_: TokenStream) -> Self {
        self.seal_generics = generics;
        self.seal_where = where_;
        self
    }

    /// Generics + where-clause for the extension trait declaration and its
    /// impl. `decl` is what appears after the trait name in the declaration
    /// (typically `seal_generics` plus a leading `__V`); `use_` is what
    /// appears in `impl Name<…> for …`.
    pub(crate) fn trait_generics(
        mut self,
        decl: TokenStream,
        use_: TokenStream,
        where_: TokenStream,
    ) -> Self {
        self.trait_generics_decl = decl;
        self.trait_generics_use = use_;
        self.trait_where = where_;
        self
    }

    /// Visibility of the extension trait itself. Defaults to inherited.
    pub(crate) fn trait_visibility(mut self, vis: Visibility) -> Self {
        self.trait_visibility = vis;
        self
    }

    /// Return the witness trait ident for `vis`, recording a new bucket on
    /// first use. Repeat calls for the same visibility reuse the previously
    /// minted ident.
    pub(crate) fn push_witness(&mut self, vis: &Visibility) -> Ident {
        if let Some(b) = self.buckets.iter().find(|b| &b.vis == vis) {
            return b.witness.clone();
        }
        let suffix = visibility_suffix(vis);
        let marker = Ident::new(&format!("__{}Marker{}", self.prefix, suffix), self.span);
        let witness = Ident::new(&format!("__{}VisibleIn{}", self.prefix, suffix), self.span);
        self.buckets.push(Bucket {
            vis: vis.clone(),
            marker,
            witness: witness.clone(),
        });
        witness
    }

    /// Queue a method for the extension trait. `sig` is everything before the
    /// semicolon / body (e.g. `fn foo(self) -> X where Self: W<__V>`); `body`
    /// is the `{ … }` block that follows in the impl.
    pub(crate) fn push_method(&mut self, sig: TokenStream, body: TokenStream) {
        self.items.push((quote! { #sig; }, quote! { #sig #body }));
    }

    /// Queue a non-fn associated item (const / type). `trait_item` appears in
    /// the trait decl, `impl_item` in the trait impl.
    pub(crate) fn push_assoc(&mut self, trait_item: TokenStream, impl_item: TokenStream) {
        self.items.push((trait_item, impl_item));
    }

    /// Consume the builder and render the full expansion: seal scaffolding,
    /// extension trait declaration, and extension trait impl.
    pub(crate) fn into_tokens(self) -> TokenStream {
        let SealBuilder {
            prefix,
            span,
            store_ty,
            seal_generics,
            seal_where,
            trait_visibility,
            trait_name,
            trait_generics_decl,
            trait_generics_use,
            trait_where,
            buckets,
            items,
        } = self;
        let sealed = Ident::new(&format!("__{}Sealed", prefix), span);

        let markers = buckets.iter().map(|b| {
            let Bucket { vis, marker, .. } = b;
            quote! {
                #[doc(hidden)]
                #[allow(non_camel_case_types)]
                #vis struct #marker;
            }
        });
        let witness_traits = buckets.iter().map(|b| {
            let witness = &b.witness;
            quote! {
                #[doc(hidden)]
                #[allow(non_camel_case_types)]
                trait #witness<__V> {}
            }
        });
        let witness_impls = buckets.iter().map(|b| {
            let Bucket {
                marker, witness, ..
            } = b;
            quote! {
                impl #seal_generics #witness<#marker> for #store_ty #seal_where {}
            }
        });
        let (trait_items, impl_items): (Vec<_>, Vec<_>) = items.into_iter().unzip();

        quote! {
            #(#markers)*
            #(#witness_traits)*
            #(#witness_impls)*

            #[doc(hidden)]
            #[allow(non_camel_case_types)]
            trait #sealed {}
            impl #seal_generics #sealed for #store_ty #seal_where {}

            #[allow(private_bounds)]
            #trait_visibility trait #trait_name #trait_generics_decl: #sealed #trait_where {
                #(#trait_items)*
            }

            #[allow(private_bounds)]
            impl #trait_generics_decl #trait_name #trait_generics_use for #store_ty #trait_where {
                #(#impl_items)*
            }
        }
    }
}
