extern crate proc_macro;

use std::collections::HashSet;

use proc_macro::TokenStream;
use quote::quote;
use syn::{ItemImpl, Type};

/// A helper attribute for deriving `State` for a struct.
#[proc_macro_attribute]
pub fn partial_derive_pass(_: TokenStream, input: TokenStream) -> TokenStream {
    let impl_block: syn::ItemImpl = syn::parse(input).unwrap();

    let parent_dependencies = impl_block
        .items
        .iter()
        .find_map(|item| {
            if let syn::ImplItem::Type(syn::ImplItemType {
                attrs, ident, ty, ..
            }) = item
            {
                (ident == "ParentDependencies").then_some(ty)
            } else {
                None
            }
        })
        .unwrap();
    let child_dependencies = impl_block
        .items
        .iter()
        .find_map(|item| {
            if let syn::ImplItem::Type(syn::ImplItemType {
                attrs, ident, ty, ..
            }) = item
            {
                (ident == "ChildDependencies").then_some(ty)
            } else {
                None
            }
        })
        .unwrap();
    let node_dependencies = impl_block
        .items
        .iter()
        .find_map(|item| {
            if let syn::ImplItem::Type(syn::ImplItemType {
                attrs, ident, ty, ..
            }) = item
            {
                (ident == "NodeDependencies").then_some(ty)
            } else {
                None
            }
        })
        .unwrap();

    let this_type = &impl_block.self_ty;

    let mut combined_dependencies = HashSet::new();

    let parent_dependencies = match parent_dependencies {
        Type::Tuple(tuple) => {
            let mut parent_dependencies = Vec::new();
            for type_ in &tuple.elems {
                combined_dependencies.insert(type_.clone());
                parent_dependencies.push(type_.clone());
            }
            parent_dependencies
        }
        _ => panic!("ParentDependencies must be a tuple"),
    };
    let child_dependencies = match child_dependencies {
        Type::Tuple(tuple) => {
            let mut child_dependencies = Vec::new();
            for type_ in &tuple.elems {
                combined_dependencies.insert(type_.clone());
                child_dependencies.push(type_.clone());
            }
            child_dependencies
        }
        _ => panic!("ChildDependencies must be a tuple"),
    };
    let node_dependencies = match node_dependencies {
        Type::Tuple(tuple) => {
            let mut node_dependencies = Vec::new();
            for type_ in &tuple.elems {
                combined_dependencies.insert(type_.clone());
                node_dependencies.push(type_.clone());
            }
            node_dependencies
        }
        _ => panic!("NodeDependencies must be a tuple"),
    };
    combined_dependencies.insert(*this_type.clone());

    let combined_dependencies: Vec<_> = combined_dependencies.into_iter().collect();
    let parent_dependancies_idxes: Vec<_> = combined_dependencies
        .iter()
        .enumerate()
        .filter_map(|(i, ident)| parent_dependencies.contains(ident).then_some(i))
        .collect();
    let child_dependencies_idxes: Vec<_> = combined_dependencies
        .iter()
        .enumerate()
        .filter_map(|(i, ident)| child_dependencies.contains(ident).then_some(i))
        .collect();
    let node_dependencies_idxes: Vec<_> = combined_dependencies
        .iter()
        .enumerate()
        .filter_map(|(i, ident)| node_dependencies.contains(ident).then_some(i))
        .collect();
    let this_type_idx = node_dependencies
        .iter()
        .enumerate()
        .find_map(|(i, ident)| (&**this_type == ident).then_some(i))
        .unwrap();

    let combined_dependencies = combined_dependencies.iter().map(|ident| {
        if ident == &**this_type {
            quote! {shipyard::ViewMut<#ident>}
        } else {
            quote! {shipyard::View<#ident>}
        }
    });
    let combined_dependencies = quote!((#(#combined_dependencies),*));

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

    // if the mutable borrow on the current type overlaps with the child or parent dependancies we need to use apply

    quote!(
        #(#attrs)*
        #defaultness #unsafety #impl_token #impl_token #generics #trait_ #for_ #self_ty {
            #(#items)*

            fn workload_system(type_id: TypeId, dependants: FxHashSet<TypeId>, pass_direction: PassDirection) -> WorkloadSystem {
                use shipyard::IntoWorkloadSystem;

                move |data: #combined_dependencies, run_view: RunPassView| {
                    shipyard::run(type_id, dependants, pass_direction, run_view, |id: NodeId, ctx: &SendAnyMap| {
                        // get all of the states from the tree view
                        // Safety: No node has itself as a parent or child.
                        let myself: SlabEntry<'static, Self> = unsafe {
                            std::mem::transmute(tree.get_slab_mut::<Self>().unwrap().entry(node_id))
                        };
                        let node_data = tree.get_single::<NodeType<V>>(node_id).unwrap();
                        let node = tree.get::<Self::NodeDependencies>(node_id).unwrap();
                        let children = tree.children::<Self::ChildDependencies>(node_id);
                        let parent = tree.parent::<Self::ParentDependencies>(node_id);

                        let view = NodeView::new(node_id, node_data, &node_mask);
                        if myself.value.is_none() {
                            *myself.value = Some(Self::create(view, node, parent, children, context));
                            true
                        } else {
                            myself
                                .value
                                .as_mut()
                                .unwrap()
                                .update(view, node, parent, children, context)
                        }
                    })
                }.into_workload_system()
            }
        }
    )
    .into()
}

// pub trait State<V: FromAnyValue + Send + Sync = ()>: Any + Send + Sync {
//     /// This is a tuple of (T: Pass, ..) of states read from the parent required to run this pass
//     type ParentDependencies: Dependancy;
//     /// This is a tuple of (T: Pass, ..) of states read from the children required to run this pass
//     type ChildDependencies: Dependancy;
//     /// This is a tuple of (T: Pass, ..) of states read from the node required to run this pass
//     type NodeDependencies: Dependancy;
//     /// A tuple of all the dependencies combined
//     type CombinedDependencies: Dependancy;
//     /// This is a mask of what aspects of the node are required to run this pass
//     const NODE_MASK: NodeMaskBuilder<'static>;
//
//     /// Update this state in a node, returns if the state was updated
//     fn update<'a>(
//         &mut self,
//         node_view: NodeView<V>,
//         node: <Self::NodeDependencies as Dependancy>::ElementBorrowed<'a>,
//         parent: Option<<Self::ParentDependencies as Dependancy>::ElementBorrowed<'a>>,
//         children: Option<Vec<<Self::ChildDependencies as Dependancy>::ElementBorrowed<'a>>>,
//         context: &SendAnyMap,
//     ) -> bool;
//
//     /// Create a new instance of this state
//     fn create<'a>(
//         node_view: NodeView<V>,
//         node: <Self::NodeDependencies as Dependancy>::ElementBorrowed<'a>,
//         parent: Option<<Self::ParentDependencies as Dependancy>::ElementBorrowed<'a>>,
//         children: Option<Vec<<Self::ChildDependencies as Dependancy>::ElementBorrowed<'a>>>,
//         context: &SendAnyMap,
//     ) -> Self;
//
//     /// Create a workload system for this state
//     fn workload_system(dependants: FxHashSet<TypeId>) -> WorkloadSystem;
// }
