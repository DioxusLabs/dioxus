extern crate proc_macro;

use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{
    self,
    parse::{Parse, ParseStream},
    Field, Ident, Token, Type,
};

#[derive(PartialEq)]
enum DepKind {
    NodeDepState,
    ChildDepState,
    ParentDepState,
}

// macro that streams data from the State for any attributes that end with _
#[proc_macro_derive(State, attributes(node_dep_state, child_dep_state, parent_dep_state))]
pub fn state_macro_derive(input: TokenStream) -> TokenStream {
    let ast = syn::parse(input).unwrap();
    impl_derive_macro(&ast)
}

fn impl_derive_macro(ast: &syn::DeriveInput) -> TokenStream {
    let type_name = &ast.ident;
    let fields: Vec<_> = match &ast.data {
        syn::Data::Struct(data) => match &data.fields {
            syn::Fields::Named(e) => &e.named,
            syn::Fields::Unnamed(_) => todo!("unnamed fields"),
            syn::Fields::Unit => todo!("unit fields"),
        }
        .iter()
        .collect(),
        _ => unimplemented!(),
    };
    let strct = Struct::parse(&fields);
    let state_strct = StateStruct::parse(&fields, &strct);
    let node_dep_state_fields = quote::__private::TokenStream::from_iter(
        state_strct
            .state_members
            .iter()
            .filter(|f| f.dep_kind == DepKind::NodeDepState)
            .map(|f| {
                let ty_id = &f.type_id();
                let reduce = &f.reduce_self();
                quote! {
                    else if ty == #ty_id {
                        #reduce
                    }
                }
            }),
    );
    let child_dep_state_fields = quote::__private::TokenStream::from_iter(
        state_strct
            .state_members
            .iter()
            .filter(|f| f.dep_kind == DepKind::ChildDepState)
            .map(|f| {
                let ty_id = &f.type_id();
                let reduce = &f.reduce_self();
                quote! {
                    else if ty == #ty_id {
                        #reduce
                    }
                }
            }),
    );
    let parent_dep_state_fields = quote::__private::TokenStream::from_iter(
        state_strct
            .state_members
            .iter()
            .filter(|f| f.dep_kind == DepKind::ParentDepState)
            .map(|f| {
                let ty_id = &f.type_id();
                let reduce = &f.reduce_self();
                quote! {
                    else if ty == #ty_id {
                        #reduce
                    }
                }
            }),
    );

    let node_types = state_strct
        .state_members
        .iter()
        .filter(|f| f.dep_kind == DepKind::NodeDepState)
        .map(|f| f.type_id());
    let child_types = state_strct
        .state_members
        .iter()
        .filter(|f| f.dep_kind == DepKind::ChildDepState)
        .map(|f| f.type_id());
    let parent_types = state_strct
        .state_members
        .iter()
        .filter(|f| f.dep_kind == DepKind::ParentDepState)
        .map(|f| f.type_id());

    let type_name_str = type_name.to_string();

    let gen = quote! {
        impl State for #type_name{
            fn update_node_dep_state(&mut self, ty: std::any::TypeId, node: dioxus_native_core::real_dom_new_api::NodeRef, ctx: &anymap::AnyMap){
                use dioxus_native_core::real_dom_new_api::NodeDepState;
                if false {}
                #node_dep_state_fields
                else{
                    panic!("{:?} not in {}", ty, #type_name_str);
                }
            }

            fn update_parent_dep_state(&mut self, ty: std::any::TypeId, node: dioxus_native_core::real_dom_new_api::NodeRef, parent: &Self, ctx: &anymap::AnyMap){
                use dioxus_native_core::real_dom_new_api::ParentDepState;
                if false {}
                #parent_dep_state_fields
                else{
                    panic!("{:?} not in {}", ty, #type_name_str);
                }
            }

            fn update_child_dep_state(&mut self, ty: std::any::TypeId, node: dioxus_native_core::real_dom_new_api::NodeRef, children: Vec<&Self>, ctx: &anymap::AnyMap){
                use dioxus_native_core::real_dom_new_api::ChildDepState;
                if false {}
                #child_dep_state_fields
                else{
                    panic!("{:?} not in {}", ty, #type_name_str);
                }
            }

            fn child_dep_types(&self) -> Vec<std::any::TypeId>{
                // todo: order should depend on order of dependencies
                vec![
                    #(#child_types,)*
                ]
            }

            fn parent_dep_types(&self) -> Vec<std::any::TypeId>{
                // todo: order should depend on order of dependencies
                vec![
                    #(#parent_types,)*
                ]
            }

            fn node_dep_types(&self) -> Vec<std::any::TypeId>{
                vec![
                    #(#node_types,)*
                ]
            }
        }
    };
    gen.into()
}

struct Struct {
    members: Vec<Member>,
}

impl Struct {
    fn parse(fields: &[&Field]) -> Self {
        let members = fields.iter().filter_map(|f| Member::parse(f)).collect();
        Self { members }
    }
}

struct StateStruct<'a> {
    state_members: Vec<StateMember<'a>>,
}

impl<'a> StateStruct<'a> {
    fn parse(fields: &[&'a Field], strct: &'a Struct) -> Self {
        let state_members = strct
            .members
            .iter()
            .zip(fields.iter())
            .filter_map(|(m, f)| StateMember::parse(f, m, &strct))
            .collect();
        Self { state_members }
    }
}

struct DepTypes {
    ctx_ty: Option<Type>,
    dep_ty: Option<Type>,
}

impl Parse for DepTypes {
    fn parse(input: ParseStream) -> Result<DepTypes, syn::Error> {
        let ctx_ty = input.parse::<Type>().ok();
        let comma = input.parse::<Token![,]>().ok();
        let dep_ty = input.parse::<Type>().ok();
        let dep_ty = comma.and(dep_ty);
        Ok(DepTypes { ctx_ty, dep_ty })
    }
}

struct Member {
    ty: Type,
    ident: Ident,
}

impl Member {
    fn parse(field: &Field) -> Option<Self> {
        Some(Self {
            ty: field.ty.clone(),
            ident: field.ident.as_ref()?.clone(),
        })
    }
}

struct StateMember<'a> {
    mem: &'a Member,
    dep_kind: DepKind,
    dep_mem: Option<&'a Member>,
    ctx_ty: Option<Type>,
}

impl<'a> StateMember<'a> {
    fn parse(field: &Field, mem: &'a Member, parent: &'a Struct) -> Option<StateMember<'a>> {
        field.attrs.iter().find_map(|a| {
            let dep_kind = a
                .path
                .get_ident()
                .map(|i| match i.to_string().as_str() {
                    "node_dep_state" => Some(DepKind::NodeDepState),
                    "child_dep_state" => Some(DepKind::ChildDepState),
                    "parent_dep_state" => Some(DepKind::ParentDepState),
                    _ => None,
                })
                .flatten()?;
            let deps: DepTypes = a.parse_args().ok()?;

            Some(Self {
                mem,
                dep_kind,
                dep_mem: deps
                    .dep_ty
                    .map(|ty| parent.members.iter().find(|m| m.ty == ty))
                    .flatten(),
                ctx_ty: deps.ctx_ty,
            })
        })
    }

    fn reduce_self(&self) -> quote::__private::TokenStream {
        let ident = &self.mem.ident;
        let get_ctx = if let Some(ctx_ty) = &self.ctx_ty {
            let msg = ctx_ty.to_token_stream().to_string() + " not found in context";
            quote! {ctx.get().expect(#msg)}
        } else {
            quote! {&()}
        };
        if let Some(dep_ident) = &self.dep_mem.map(|m| &m.ident) {
            match self.dep_kind {
                DepKind::NodeDepState => {
                    quote!(self.#ident.reduce(node, #get_ctx);)
                }
                DepKind::ChildDepState => {
                    quote!(self.#ident.reduce(node, children.iter().map(|s| &s.#dep_ident).collect(), #get_ctx);)
                }
                DepKind::ParentDepState => {
                    quote!(self.#ident.reduce(node, &parent.#dep_ident, #get_ctx);)
                }
            }
        } else {
            match self.dep_kind {
                DepKind::NodeDepState => {
                    quote!(self.#ident.reduce(node, #get_ctx);)
                }
                DepKind::ChildDepState => {
                    quote!(self.#ident.reduce(node, &(), #get_ctx);)
                }
                DepKind::ParentDepState => {
                    quote!(self.#ident.reduce(node, &(), #get_ctx);)
                }
            }
        }
    }

    fn type_id(&self) -> quote::__private::TokenStream {
        let ty = &self.mem.ty;
        quote!(std::any::TypeId::of::<#ty>())
    }
}
