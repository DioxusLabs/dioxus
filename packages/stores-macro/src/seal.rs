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

/// Inputs the caller provides up front. Generics are pre-tokenized so the
/// builder is agnostic to how the caller built them.
pub(crate) struct SealConfig {
    /// Ident prefix for every generated type, e.g. `"TodoItemStore"` →
    /// `__TodoItemStoreSealed`, `__TodoItemStoreMarkerPub`,
    /// `__TodoItemStoreVisibleInPub`.
    pub prefix: String,
    /// Span attached to every generated ident.
    pub span: Span,
    /// The store type the extension trait is implemented for, e.g.
    /// `dioxus_stores::Store<TodoItem, __Lens>`.
    pub store_ty: TokenStream,
    /// Impl generics for the marker / witness / sealed blanket impls (does not
    /// include `__V`, which lives only on the extension trait).
    pub seal_generics: TokenStream,
    /// Where-clause for those blanket impls.
    pub seal_where: TokenStream,
    /// Visibility of the extension trait itself.
    pub trait_visibility: Visibility,
    /// Name of the extension trait.
    pub trait_name: Ident,
    /// Generics as they appear on the trait declaration and the trait impl
    /// (usually `seal_generics` plus a leading `__V`).
    pub trait_generics_decl: TokenStream,
    /// Generics as they appear after the trait name in `impl Name<…> for …`.
    pub trait_generics_use: TokenStream,
    /// Where-clause on the trait decl + impl.
    pub trait_where: TokenStream,
}

struct Bucket {
    vis: Visibility,
    marker: Ident,
    witness: Ident,
}

pub(crate) struct SealBuilder {
    cfg: SealConfig,
    sealed: Ident,
    buckets: Vec<Bucket>,
    /// `(trait_side, impl_side)` for each associated item. Fn methods push
    /// `(sig;, sig body)`; consts / types push the same tokens on both sides.
    items: Vec<(TokenStream, TokenStream)>,
}

impl SealBuilder {
    pub fn new(cfg: SealConfig) -> Self {
        let sealed = Ident::new(&format!("__{}Sealed", cfg.prefix), cfg.span);
        Self {
            cfg,
            sealed,
            buckets: Vec::new(),
            items: Vec::new(),
        }
    }

    /// Return the witness trait ident for `vis`, recording a new bucket on
    /// first use. Repeat calls for the same visibility reuse the previously
    /// minted ident.
    pub fn push_witness(&mut self, vis: &Visibility) -> Ident {
        if let Some(b) = self.buckets.iter().find(|b| &b.vis == vis) {
            return b.witness.clone();
        }
        let suffix = visibility_suffix(vis);
        let marker = Ident::new(
            &format!("__{}Marker{}", self.cfg.prefix, suffix),
            self.cfg.span,
        );
        let witness = Ident::new(
            &format!("__{}VisibleIn{}", self.cfg.prefix, suffix),
            self.cfg.span,
        );
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
    pub fn push_method(&mut self, sig: TokenStream, body: TokenStream) {
        self.items.push((quote! { #sig; }, quote! { #sig #body }));
    }

    /// Queue a non-fn associated item (const / type). `trait_item` appears in
    /// the trait decl, `impl_item` in the trait impl.
    pub fn push_assoc(&mut self, trait_item: TokenStream, impl_item: TokenStream) {
        self.items.push((trait_item, impl_item));
    }

    /// Consume the builder and render the full expansion: seal scaffolding,
    /// extension trait declaration, and extension trait impl.
    pub fn into_tokens(self) -> TokenStream {
        let SealBuilder {
            cfg,
            sealed,
            buckets,
            items,
        } = self;
        let SealConfig {
            store_ty,
            seal_generics,
            seal_where,
            trait_visibility,
            trait_name,
            trait_generics_decl,
            trait_generics_use,
            trait_where,
            ..
        } = cfg;

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
