extern crate proc_macro;

use std::collections::BTreeMap;

use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{
    self, bracketed,
    parse::{Parse, ParseStream, Result},
    parse_macro_input,
    punctuated::Punctuated,
    token::Paren,
    Field, Ident, LitStr, Token, Type, TypeTuple,
};

struct StrSlice {
    map: BTreeMap<String, LitStr>,
}

impl Parse for StrSlice {
    fn parse(input: ParseStream) -> Result<Self> {
        let content;
        bracketed!(content in input);
        let mut map = BTreeMap::new();
        while let Ok(s) = content.parse::<LitStr>() {
            map.insert(s.value(), s);
            #[allow(unused_must_use)]
            {
                content.parse::<Token![,]>();
            }
        }
        Ok(StrSlice { map })
    }
}

#[proc_macro]
pub fn sorted_str_slice(input: TokenStream) -> TokenStream {
    let slice: StrSlice = parse_macro_input!(input as StrSlice);
    let strings = slice.map.values();
    quote!([#(#strings, )*]).into()
}

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
        .map(|f| &f.mem.ty);
    let child_types = state_strct
        .state_members
        .iter()
        .filter(|f| f.dep_kind == DepKind::ChildDepState)
        .map(|f| &f.mem.ty);
    let parent_types = state_strct
        .state_members
        .iter()
        .filter(|f| f.dep_kind == DepKind::ParentDepState)
        .map(|f| &f.mem.ty);

    let type_name_str = type_name.to_string();

    let gen = quote! {
        impl State for #type_name{
        fn update_node_dep_state<'a>(&'a mut self, ty: std::any::TypeId, node: &'a dioxus_core::VNode<'a>, vdom: &'a dioxus_core::VirtualDom, ctx: &anymap::AnyMap) -> bool{
                use dioxus_native_core::state::NodeDepState as _;
                // println!("called update_node_dep_state with ty: {:?}", ty);
                if false {
                    unreachable!();
                }
                #node_dep_state_fields
                else{
                    panic!("{:?} not in {}", ty, #type_name_str)
                }
            }

            fn update_parent_dep_state<'a>(&'a mut self, ty: std::any::TypeId, node: &'a dioxus_core::VNode<'a>, vdom: &'a dioxus_core::VirtualDom, parent: Option<&Self>, ctx: &anymap::AnyMap) -> bool{
                use dioxus_native_core::state::ParentDepState as _;
                // println!("called update_parent_dep_state with ty: {:?}", ty);
                if false {
                    unreachable!();
                }
                #parent_dep_state_fields
                else{
                    panic!("{:?} not in {}", ty, #type_name_str)
                }
            }

            fn update_child_dep_state<'a>(&'a mut self, ty: std::any::TypeId, node: &'a dioxus_core::VNode<'a>, vdom: &'a dioxus_core::VirtualDom, children: &[&Self], ctx: &anymap::AnyMap) -> bool{
                use dioxus_native_core::state::ChildDepState as _;
                // println!("called update_child_dep_state with ty: {:?}", ty);
                if false {
                    unreachable!()
                }
                #child_dep_state_fields
                else{
                    panic!("{:?} not in {}", ty, #type_name_str)
                }
            }

            fn child_dep_types(&self, mask: &dioxus_native_core::state::NodeMask) -> Vec<std::any::TypeId>{
                let mut dep_types = Vec::new();
                #(if #child_types::NODE_MASK.overlaps(mask) {
                    dep_types.push(std::any::TypeId::of::<#child_types>());
                })*
                dep_types
            }

            fn parent_dep_types(&self, mask: &dioxus_native_core::state::NodeMask) -> Vec<std::any::TypeId>{
                let mut dep_types = Vec::new();
                #(if #parent_types::NODE_MASK.overlaps(mask) {
                    dep_types.push(std::any::TypeId::of::<#parent_types>());
                })*
                dep_types
            }

            fn node_dep_types(&self, mask: &dioxus_native_core::state::NodeMask) -> Vec<std::any::TypeId>{
                let mut dep_types = Vec::new();
                #(if #node_types::NODE_MASK.overlaps(mask) {
                    dep_types.push(std::any::TypeId::of::<#node_types>());
                })*
                dep_types
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

        // todo: sort members

        Self { state_members }
    }
}

struct DepTypes {
    ctx_ty: Option<Type>,
    dep_ty: Option<Type>,
}

impl Parse for DepTypes {
    fn parse(input: ParseStream) -> Result<Self> {
        let dep_ty = input.parse().ok();
        let comma: Option<Token![,]> = input.parse().ok();
        let ctx_ty = input.parse().ok();
        Ok(Self {
            ctx_ty: comma.and(ctx_ty),
            dep_ty,
        })
    }
}

struct NodeDepTypes {
    ctx_ty: Option<Type>,
}

impl Parse for NodeDepTypes {
    fn parse(input: ParseStream) -> Result<Self> {
        let ctx_ty = input.parse().ok();
        Ok(Self { ctx_ty })
    }
}

impl From<NodeDepTypes> for DepTypes {
    fn from(node_dep_types: NodeDepTypes) -> Self {
        Self {
            ctx_ty: node_dep_types.ctx_ty,
            dep_ty: None,
        }
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
            let deps: DepTypes = match dep_kind {
                DepKind::NodeDepState => a.parse_args::<NodeDepTypes>().ok()?.into(),
                _ => a.parse_args().ok()?,
            };

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
            if ctx_ty
                == &Type::Tuple(TypeTuple {
                    paren_token: Paren {
                        span: quote::__private::Span::call_site(),
                    },
                    elems: Punctuated::new(),
                })
            {
                quote! {&()}
            } else {
                let msg = ctx_ty.to_token_stream().to_string() + " not found in context";
                quote! {ctx.get().expect(#msg)}
            }
        } else {
            quote! {&()}
        };
        let ty = &self.mem.ty;
        let node_view = quote!(NodeView::new(node, #ty::NODE_MASK, vdom));
        if let Some(dep_ident) = &self.dep_mem.map(|m| &m.ident) {
            match self.dep_kind {
                DepKind::NodeDepState => {
                    quote!(self.#ident.reduce(#node_view, #get_ctx))
                }
                DepKind::ChildDepState => {
                    quote!(self.#ident.reduce(#node_view, children.iter().map(|s| &s.#dep_ident), #get_ctx))
                }
                DepKind::ParentDepState => {
                    quote!(self.#ident.reduce(#node_view, parent.as_ref().map(|p| &p.#dep_ident), #get_ctx))
                }
            }
        } else {
            match self.dep_kind {
                DepKind::NodeDepState => {
                    quote!(self.#ident.reduce(#node_view, #get_ctx))
                }
                DepKind::ChildDepState => {
                    quote!(self.#ident.reduce(#node_view, &(), #get_ctx))
                }
                DepKind::ParentDepState => {
                    quote!(self.#ident.reduce(#node_view, Some(&()), #get_ctx))
                }
            }
        }
    }

    fn type_id(&self) -> quote::__private::TokenStream {
        let ty = &self.mem.ty;
        quote!({
            let type_id = std::any::TypeId::of::<#ty>();
            type_id
        })
    }
}
