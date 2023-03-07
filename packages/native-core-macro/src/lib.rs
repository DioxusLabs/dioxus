extern crate proc_macro;

use std::collections::HashSet;

use proc_macro::TokenStream;
use quote::quote;
use syn::Type;

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

    match parent_dependencies {
        Type::Tuple(tuple) => {
            for type_ in &tuple.elems {
                combined_dependencies.insert(type_.clone());
            }
        }
        _ => panic!("ParentDependencies must be a tuple"),
    }
    match child_dependencies {
        Type::Tuple(tuple) => {
            for type_ in &tuple.elems {
                combined_dependencies.insert(type_.clone());
            }
        }
        _ => panic!("ChildDependencies must be a tuple"),
    }
    match node_dependencies {
        Type::Tuple(tuple) => {
            for type_ in &tuple.elems {
                combined_dependencies.insert(type_.clone());
            }
        }
        _ => panic!("NodeDependencies must be a tuple"),
    }
    combined_dependencies.insert(*this_type.clone());

    let combined_dependencies = combined_dependencies.into_iter();
    let combined_dependencies = quote!((#(#combined_dependencies),*));

    quote!(#impl_block).into()
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
