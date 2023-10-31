#![doc = include_str!("../README.md")]
#![doc(html_logo_url = "https://avatars.githubusercontent.com/u/79236386")]
#![doc(html_favicon_url = "https://avatars.githubusercontent.com/u/79236386")]

extern crate proc_macro;

use std::collections::HashSet;

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, ItemImpl, Type, TypePath, TypeTuple};

/// A helper attribute for deriving `State` for a struct.
#[proc_macro_attribute]
pub fn partial_derive_state(_: TokenStream, input: TokenStream) -> TokenStream {
    let impl_block: syn::ItemImpl = parse_macro_input!(input as syn::ItemImpl);

    let has_create_fn = impl_block
        .items
        .iter()
        .any(|item| matches!(item, syn::ImplItem::Fn(method) if method.sig.ident == "create"));

    let parent_dependencies = impl_block
        .items
        .iter()
        .find_map(|item| {
            if let syn::ImplItem::Type(syn::ImplItemType { ident, ty, .. }) = item {
                (ident == "ParentDependencies").then_some(ty)
            } else {
                None
            }
        })
        .expect("ParentDependencies must be defined");
    let child_dependencies = impl_block
        .items
        .iter()
        .find_map(|item| {
            if let syn::ImplItem::Type(syn::ImplItemType { ident, ty, .. }) = item {
                (ident == "ChildDependencies").then_some(ty)
            } else {
                None
            }
        })
        .expect("ChildDependencies must be defined");
    let node_dependencies = impl_block
        .items
        .iter()
        .find_map(|item| {
            if let syn::ImplItem::Type(syn::ImplItemType { ident, ty, .. }) = item {
                (ident == "NodeDependencies").then_some(ty)
            } else {
                None
            }
        })
        .expect("NodeDependencies must be defined");

    let this_type = &impl_block.self_ty;
    let this_type = extract_type_path(this_type)
        .unwrap_or_else(|| panic!("Self must be a type path, found {}", quote!(#this_type)));

    let mut combined_dependencies = HashSet::new();

    let self_path: TypePath = syn::parse_quote!(Self);

    let parent_dependencies = match extract_tuple(parent_dependencies) {
        Some(tuple) => {
            let mut parent_dependencies = Vec::new();
            for type_ in &tuple.elems {
                let mut type_ = extract_type_path(type_).unwrap_or_else(|| {
                    panic!(
                        "ParentDependencies must be a tuple of type paths, found {}",
                        quote!(#type_)
                    )
                });
                if type_ == self_path {
                    type_ = this_type.clone();
                }
                combined_dependencies.insert(type_.clone());
                parent_dependencies.push(type_);
            }
            parent_dependencies
        }
        _ => panic!(
            "ParentDependencies must be a tuple, found {}",
            quote!(#parent_dependencies)
        ),
    };
    let child_dependencies = match extract_tuple(child_dependencies) {
        Some(tuple) => {
            let mut child_dependencies = Vec::new();
            for type_ in &tuple.elems {
                let mut type_ = extract_type_path(type_).unwrap_or_else(|| {
                    panic!(
                        "ChildDependencies must be a tuple of type paths, found {}",
                        quote!(#type_)
                    )
                });
                if type_ == self_path {
                    type_ = this_type.clone();
                }
                combined_dependencies.insert(type_.clone());
                child_dependencies.push(type_);
            }
            child_dependencies
        }
        _ => panic!(
            "ChildDependencies must be a tuple, found {}",
            quote!(#child_dependencies)
        ),
    };
    let node_dependencies = match extract_tuple(node_dependencies) {
        Some(tuple) => {
            let mut node_dependencies = Vec::new();
            for type_ in &tuple.elems {
                let mut type_ = extract_type_path(type_).unwrap_or_else(|| {
                    panic!(
                        "NodeDependencies must be a tuple of type paths, found {}",
                        quote!(#type_)
                    )
                });
                if type_ == self_path {
                    type_ = this_type.clone();
                }
                combined_dependencies.insert(type_.clone());
                node_dependencies.push(type_);
            }
            node_dependencies
        }
        _ => panic!(
            "NodeDependencies must be a tuple, found {}",
            quote!(#node_dependencies)
        ),
    };
    combined_dependencies.insert(this_type.clone());

    let combined_dependencies: Vec<_> = combined_dependencies.into_iter().collect();
    let parent_dependancies_idxes: Vec<_> = parent_dependencies
        .iter()
        .filter_map(|ident| combined_dependencies.iter().position(|i| i == ident))
        .collect();
    let child_dependencies_idxes: Vec<_> = child_dependencies
        .iter()
        .filter_map(|ident| combined_dependencies.iter().position(|i| i == ident))
        .collect();
    let node_dependencies_idxes: Vec<_> = node_dependencies
        .iter()
        .filter_map(|ident| combined_dependencies.iter().position(|i| i == ident))
        .collect();
    let this_type_idx = combined_dependencies
        .iter()
        .enumerate()
        .find_map(|(i, ident)| (this_type == *ident).then_some(i))
        .unwrap();
    let this_view = format_ident!("__data{}", this_type_idx);

    let combined_dependencies_quote = combined_dependencies.iter().map(|ident| {
        if ident == &this_type {
            quote! {shipyard::ViewMut<#ident>}
        } else {
            quote! {shipyard::View<#ident>}
        }
    });
    let combined_dependencies_quote = quote!((#(#combined_dependencies_quote,)*));

    let ItemImpl {
        attrs,
        defaultness,
        unsafety,
        impl_token,
        generics,
        trait_,
        self_ty,
        items,
        ..
    } = impl_block;
    let for_ = trait_.as_ref().map(|t| t.2);
    let trait_ = trait_.map(|t| t.1);

    let split_views: Vec<_> = (0..combined_dependencies.len())
        .map(|i| {
            let ident = format_ident!("__data{}", i);
            if i == this_type_idx {
                quote! {mut #ident}
            } else {
                quote! {#ident}
            }
        })
        .collect();

    let node_view = node_dependencies_idxes
        .iter()
        .map(|i| format_ident!("__data{}", i))
        .collect::<Vec<_>>();
    let get_node_view = {
        if node_dependencies.is_empty() {
            quote! {
                let raw_node = ();
            }
        } else {
            let temps = (0..node_dependencies.len())
                .map(|i| format_ident!("__temp{}", i))
                .collect::<Vec<_>>();
            quote! {
                let raw_node: (#(*const #node_dependencies,)*) = {
                    let (#(#temps,)*) = (#(&#node_view,)*).get(id).unwrap_or_else(|err| panic!("Failed to get node view {:?}", err));
                    (#(#temps as *const _,)*)
                };
            }
        }
    };
    let deref_node_view = {
        if node_dependencies.is_empty() {
            quote! {
                let node = raw_node;
            }
        } else {
            let indexes = (0..node_dependencies.len()).map(syn::Index::from);
            quote! {
                let node = unsafe { (#(dioxus_native_core::prelude::DependancyView::new(&*raw_node.#indexes),)*) };
            }
        }
    };

    let parent_view = parent_dependancies_idxes
        .iter()
        .map(|i| format_ident!("__data{}", i))
        .collect::<Vec<_>>();
    let get_parent_view = {
        if parent_dependencies.is_empty() {
            quote! {
                let raw_parent = tree.parent_id_advanced(id, Self::TRAVERSE_SHADOW_DOM).map(|_| ());
            }
        } else {
            let temps = (0..parent_dependencies.len())
                .map(|i| format_ident!("__temp{}", i))
                .collect::<Vec<_>>();
            quote! {
                let raw_parent = tree.parent_id_advanced(id, Self::TRAVERSE_SHADOW_DOM).and_then(|parent_id| {
                    let raw_parent: Option<(#(*const #parent_dependencies,)*)> = (#(&#parent_view,)*).get(parent_id).ok().map(|c| {
                        let (#(#temps,)*) = c;
                        (#(#temps as *const _,)*)
                    });
                    raw_parent
                });
            }
        }
    };
    let deref_parent_view = {
        if parent_dependencies.is_empty() {
            quote! {
                let parent = raw_parent;
            }
        } else {
            let indexes = (0..parent_dependencies.len()).map(syn::Index::from);
            quote! {
                let parent = unsafe { raw_parent.map(|raw_parent| (#(dioxus_native_core::prelude::DependancyView::new(&*raw_parent.#indexes),)*)) };
            }
        }
    };

    let child_view = child_dependencies_idxes
        .iter()
        .map(|i| format_ident!("__data{}", i))
        .collect::<Vec<_>>();
    let get_child_view = {
        if child_dependencies.is_empty() {
            quote! {
                let raw_children: Vec<_> = tree.children_ids_advanced(id, Self::TRAVERSE_SHADOW_DOM).into_iter().map(|_| ()).collect();
            }
        } else {
            let temps = (0..child_dependencies.len())
                .map(|i| format_ident!("__temp{}", i))
                .collect::<Vec<_>>();
            quote! {
                let raw_children: Vec<_> = tree.children_ids_advanced(id, Self::TRAVERSE_SHADOW_DOM).into_iter().filter_map(|id| {
                    let raw_children: Option<(#(*const #child_dependencies,)*)> = (#(&#child_view,)*).get(id).ok().map(|c| {
                        let (#(#temps,)*) = c;
                        (#(#temps as *const _,)*)
                    });
                    raw_children
                }).collect();
            }
        }
    };
    let deref_child_view = {
        if child_dependencies.is_empty() {
            quote! {
                let children = raw_children;
            }
        } else {
            let indexes = (0..child_dependencies.len()).map(syn::Index::from);
            quote! {
                let children = unsafe { raw_children.iter().map(|raw_children| (#(dioxus_native_core::prelude::DependancyView::new(&*raw_children.#indexes),)*)).collect::<Vec<_>>() };
            }
        }
    };

    let trait_generics = trait_
        .as_ref()
        .unwrap()
        .segments
        .last()
        .unwrap()
        .arguments
        .clone();

    // if a create function is defined, we don't generate one
    // otherwise we generate a default one that uses the update function and the default constructor
    let create_fn = (!has_create_fn).then(|| {
        quote! {
            fn create<'a>(
                node_view: dioxus_native_core::prelude::NodeView # trait_generics,
                node: <Self::NodeDependencies as Dependancy>::ElementBorrowed<'a>,
                parent: Option<<Self::ParentDependencies as Dependancy>::ElementBorrowed<'a>>,
                children: Vec<<Self::ChildDependencies as Dependancy>::ElementBorrowed<'a>>,
                context: &dioxus_native_core::prelude::SendAnyMap,
            ) -> Self {
                let mut myself = Self::default();
                myself.update(node_view, node, parent, children, context);
                myself
            }
        }
    });

    quote!(
        #(#attrs)*
        #defaultness #unsafety #impl_token #generics #trait_ #for_ #self_ty {
            #create_fn

            #(#items)*

            fn workload_system(type_id: std::any::TypeId, dependants: std::sync::Arc<dioxus_native_core::prelude::Dependants>, pass_direction: dioxus_native_core::prelude::PassDirection) -> dioxus_native_core::exports::shipyard::WorkloadSystem {
                use dioxus_native_core::exports::shipyard::{IntoWorkloadSystem, Get, AddComponent};
                use dioxus_native_core::tree::TreeRef;
                use dioxus_native_core::prelude::{NodeType, NodeView};

                let node_mask = Self::NODE_MASK.build();

                (move |data: #combined_dependencies_quote, run_view: dioxus_native_core::prelude::RunPassView #trait_generics| {
                    let (#(#split_views,)*) = data;
                    let tree = run_view.tree.clone();
                    let node_types = run_view.node_type.clone();
                    dioxus_native_core::prelude::run_pass(type_id, dependants.clone(), pass_direction, run_view, |id, context| {
                        let node_data: &NodeType<_> = node_types.get(id).unwrap_or_else(|err| panic!("Failed to get node type {:?}", err));
                        // get all of the states from the tree view
                        // Safety: No node has itself as a parent or child.
                        let raw_myself: Option<*mut Self> = (&mut #this_view).get(id).ok().map(|c| c as *mut _);
                        #get_node_view
                        #get_parent_view
                        #get_child_view

                        let myself: Option<&mut Self> = unsafe { raw_myself.map(|val| &mut *val) };
                        #deref_node_view
                        #deref_parent_view
                        #deref_child_view

                        let view = NodeView::new(id, node_data, &node_mask);
                        if let Some(myself) = myself {
                            myself
                                .update(view, node, parent, children, context)
                        }
                        else {
                            (&mut #this_view).add_component_unchecked(
                                id,
                                Self::create(view, node, parent, children, context));
                            true
                        }
                    })
                }).into_workload_system().unwrap()
            }
        }
    )
    .into()
}

fn extract_tuple(ty: &Type) -> Option<TypeTuple> {
    match ty {
        Type::Tuple(tuple) => Some(tuple.clone()),
        Type::Group(group) => extract_tuple(&group.elem),
        _ => None,
    }
}

fn extract_type_path(ty: &Type) -> Option<TypePath> {
    match ty {
        Type::Path(path) => Some(path.clone()),
        Type::Group(group) => extract_type_path(&group.elem),
        _ => None,
    }
}
