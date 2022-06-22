extern crate proc_macro;

mod sorted_slice;

use dioxus_native_core::state::MemberId;
use proc_macro::TokenStream;
use quote::format_ident;
use quote::{quote, ToTokens, __private::Span};
use sorted_slice::StrSlice;
use syn::{
    self,
    parse::{Parse, ParseStream, Result},
    parse_macro_input, parse_quote, Error, Field, Ident, Token, Type,
};

#[proc_macro]
pub fn sorted_str_slice(input: TokenStream) -> TokenStream {
    let slice: StrSlice = parse_macro_input!(input as StrSlice);
    let strings = slice.map.values();
    quote!([#(#strings, )*]).into()
}

#[derive(PartialEq, Debug, Clone)]
enum DepKind {
    Node,
    Child,
    Parent,
}

#[proc_macro_derive(
    State,
    attributes(node_dep_state, child_dep_state, parent_dep_state, state)
)]
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
            syn::Fields::Unit => todo!("unit structs"),
        }
        .iter()
        .collect(),
        _ => unimplemented!(),
    };
    let strct = Struct::new(type_name.clone(), &fields);
    match StateStruct::parse(&fields, &strct) {
        Ok(state_strct) => {
            let node_dep_state_fields = state_strct
                .state_members
                .iter()
                .filter(|f| f.dep_kind == DepKind::Node)
                .map(|f| f.reduce_self());
            let child_dep_state_fields = state_strct
                .state_members
                .iter()
                .filter(|f| f.dep_kind == DepKind::Child)
                .map(|f| f.reduce_self());
            let parent_dep_state_fields = state_strct
                .state_members
                .iter()
                .filter(|f| f.dep_kind == DepKind::Parent)
                .map(|f| f.reduce_self());

            let node_iter = state_strct
                .state_members
                .iter()
                .filter(|m| m.dep_kind == DepKind::Node);
            let node_ids = node_iter.clone().map(|m| m.member_id.0);
            let node_ids_clone = node_ids.clone();
            let node_types = node_iter.map(|f| &f.mem.ty);

            let child_iter = state_strct
                .state_members
                .iter()
                .filter(|m| m.dep_kind == DepKind::Child);
            let child_ids = child_iter.clone().map(|m| m.member_id.0);
            let child_ids_clone = child_ids.clone();
            let child_types = child_iter.map(|f| &f.mem.ty);

            let parent_iter = state_strct
                .state_members
                .iter()
                .filter(|m| m.dep_kind == DepKind::Parent);
            let parent_ids = parent_iter.clone().map(|m| m.member_id.0);
            let parent_ids_clone = parent_ids.clone();
            let parent_types = parent_iter.map(|f| &f.mem.ty);

            let type_name_str = type_name.to_string();

            let child_states = &state_strct.child_states;

            let member_size = state_strct.state_members.len();

            let child_state_ty = child_states.iter().map(|m| &m.ty);
            let child_state_idents: Vec<_> = child_states.iter().map(|m| &m.ident).collect();
            let sum_const_declarations = child_state_ty.clone().enumerate().map(|(i, ty)| {
                let ident = format_ident!("__{}_SUM_{}", i, type_name.to_string());
                let ident_minus = format_ident!("__{}_SUM_{}_minus", i, type_name.to_string());
                if i == 0 {
                    quote!(const #ident_minus: usize = #member_size + #ty::SIZE - 1;
                    const #ident: usize = #member_size + #ty::SIZE;)
                } else {
                    let prev_ident = format_ident!("__{}_SUM_{}", i - 1, type_name.to_string());
                    quote!(const #ident_minus: usize = #prev_ident + #ty::SIZE - 1;
                    const #ident: usize = #prev_ident + #ty::SIZE;)
                }
            });
            let sum_idents: Vec<_> = std::iter::once(quote!(#member_size))
                .chain((0..child_states.len()).map(|i| {
                    let ident = format_ident!("__{}_SUM_{}", i, type_name.to_string());
                    quote!(#ident)
                }))
                .collect();

            let child_state_ranges: Vec<_> = (0..child_state_ty.len())
                .map(|i| {
                    let current = format_ident!("__{}_SUM_{}_minus", i, type_name.to_string());
                    let previous = if i == 0 {
                        quote!(#member_size)
                    } else {
                        let ident = format_ident!("__{}_SUM_{}", i - 1, type_name.to_string());
                        quote!(#ident)
                    };
                    quote!(#previous..=#current)
                })
                .collect();

            let gen = quote! {
                #(
                    #sum_const_declarations
                )*
                impl State for #type_name{
                    const SIZE: usize = #member_size #( + #child_state_ty::SIZE)*;

                    fn update_node_dep_state<'a>(
                        &'a mut self,
                        ty: dioxus_native_core::state::MemberId,
                        node: &'a dioxus_core::VNode<'a>,
                        vdom: &'a dioxus_core::VirtualDom,
                        ctx: &anymap::AnyMap,
                    ) -> Option<dioxus_native_core::state::NodeStatesChanged>{
                        use dioxus_native_core::state::NodeDepState as _;
                        use dioxus_native_core::state::State as _;
                        match ty.0{
                            #(
                                #node_ids => #node_dep_state_fields,
                            )*
                            #(
                                #child_state_ranges => {
                                    self.#child_state_idents.update_node_dep_state(
                                        ty - #sum_idents,
                                        node,
                                        vdom,
                                        ctx,
                                    ).map(|mut changed|{
                                        for id in &mut changed.node_dep{
                                            *id += #sum_idents;
                                        }
                                        changed
                                    })
                                }
                            )*
                            _ => panic!("{:?} not in {}", ty, #type_name_str),
                        }
                    }

                    fn update_parent_dep_state<'a>(
                        &'a mut self,
                        ty: dioxus_native_core::state::MemberId,
                        node: &'a dioxus_core::VNode<'a>,
                        vdom: &'a dioxus_core::VirtualDom,
                        parent: Option<&Self>,
                        ctx: &anymap::AnyMap,
                    ) -> Option<dioxus_native_core::state::ParentStatesChanged>{
                        use dioxus_native_core::state::ParentDepState as _;
                        match ty.0{
                            #(
                                #parent_ids => #parent_dep_state_fields,
                            )*
                            #(
                                #child_state_ranges => {
                                    self.#child_state_idents.update_parent_dep_state(
                                        ty - #sum_idents,
                                        node,
                                        vdom,
                                        parent.map(|p| &p.#child_state_idents),
                                        ctx,
                                    ).map(|mut changed|{
                                        for id in &mut changed.node_dep{
                                            *id += #sum_idents;
                                        }
                                        for id in &mut changed.parent_dep{
                                            *id += #sum_idents;
                                        }
                                        changed
                                    })
                                }
                            )*
                            _ => panic!("{:?} not in {}", ty, #type_name_str),
                        }
                    }

                    fn update_child_dep_state<'a>(
                        &'a mut self,
                        ty: dioxus_native_core::state::MemberId,
                        node: &'a dioxus_core::VNode<'a>,
                        vdom: &'a dioxus_core::VirtualDom,
                        children: &[&Self],
                        ctx: &anymap::AnyMap,
                    ) -> Option<dioxus_native_core::state::ChildStatesChanged>{
                        use dioxus_native_core::state::ChildDepState as _;
                        match ty.0{
                            #(
                                #child_ids => #child_dep_state_fields,
                            )*
                            #(
                                #child_state_ranges => {
                                    self.#child_state_idents.update_child_dep_state(
                                        ty - #sum_idents,
                                        node,
                                        vdom,
                                        &children.iter().map(|p| &p.#child_state_idents).collect::<Vec<_>>(),
                                        ctx,
                                    ).map(|mut changed|{
                                        for id in &mut changed.node_dep{
                                            *id += #sum_idents;
                                        }
                                        for id in &mut changed.child_dep{
                                            *id += #sum_idents;
                                        }
                                        changed
                                    })
                                }
                            )*
                            _ => panic!("{:?} not in {}", ty, #type_name_str),
                        }
                    }

                    fn child_dep_types(&self, mask: &dioxus_native_core::node_ref::NodeMask) -> Vec<dioxus_native_core::state::MemberId>{
                        let mut dep_types = Vec::new();
                        #(if #child_types::NODE_MASK.overlaps(mask) {
                            dep_types.push(dioxus_native_core::state::MemberId(#child_ids_clone));
                        })*
                        #(
                            dep_types.extend(self.#child_state_idents.child_dep_types(mask).into_iter().map(|id| id + #sum_idents));
                        )*
                        dep_types
                    }

                    fn parent_dep_types(&self, mask: &dioxus_native_core::node_ref::NodeMask) -> Vec<dioxus_native_core::state::MemberId>{
                        let mut dep_types = Vec::new();
                        #(if #parent_types::NODE_MASK.overlaps(mask) {
                            dep_types.push(dioxus_native_core::state::MemberId(#parent_ids_clone));
                        })*
                        #(
                            dep_types.extend(self.#child_state_idents.parent_dep_types(mask).into_iter().map(|id| id + #sum_idents));
                        )*
                        dep_types
                    }

                    fn node_dep_types(&self, mask: &dioxus_native_core::node_ref::NodeMask) -> Vec<dioxus_native_core::state::MemberId>{
                        let mut dep_types = Vec::new();
                        #(if #node_types::NODE_MASK.overlaps(mask) {
                            dep_types.push(dioxus_native_core::state::MemberId(#node_ids_clone));
                        })*
                        #(
                            dep_types.extend(self.#child_state_idents.node_dep_types(mask).into_iter().map(|id| id + #sum_idents));
                        )*
                        dep_types
                    }
                }
            };
            gen.into()
        }
        Err(e) => e.into_compile_error().into(),
    }
}

struct Struct {
    name: Ident,
    members: Vec<Member>,
}

impl Struct {
    fn new(name: Ident, fields: &[&Field]) -> Self {
        let members = fields.iter().filter_map(|f| Member::parse(f)).collect();
        Self { name, members }
    }
}

struct StateStruct<'a> {
    state_members: Vec<StateMember<'a>>,
    child_states: Vec<&'a Member>,
}

impl<'a> StateStruct<'a> {
    fn parse(fields: &[&'a Field], strct: &'a Struct) -> Result<Self> {
        let mut parse_err = Ok(());
        let state_members = strct
            .members
            .iter()
            .zip(fields.iter())
            .filter_map(|(m, f)| match StateMember::parse(f, m, strct) {
                Ok(m) => m,
                Err(err) => {
                    parse_err = Err(err);
                    None
                }
            });

        let child_states = strct
            .members
            .iter()
            .zip(fields.iter())
            .filter(|(_, f)| {
                f.attrs.iter().any(|a| {
                    a.path
                        .get_ident()
                        .filter(|i| i.to_string().as_str() == "state")
                        .is_some()
                })
            })
            .map(|(m, _)| m);

        #[derive(Debug, Clone)]
        struct DepNode<'a> {
            state_mem: StateMember<'a>,
            depandants: Vec<DepNode<'a>>,
        }
        impl<'a> DepNode<'a> {
            fn new(state_mem: StateMember<'a>) -> Self {
                Self {
                    state_mem,
                    depandants: Vec::new(),
                }
            }

            /// flattens the node in pre order
            fn flatten(self) -> Vec<StateMember<'a>> {
                let DepNode {
                    state_mem,
                    depandants,
                } = self;
                let mut flat = vec![state_mem];
                for d in depandants {
                    flat.append(&mut d.flatten());
                }
                flat
            }

            fn set_ids(&mut self, current_id: &mut usize) {
                self.state_mem.member_id = dioxus_native_core::state::MemberId(*current_id);
                // if the node depends on itself, we need to add the dependency seperately
                if let Some(dep) = self.state_mem.dep_mem {
                    if dep == self.state_mem.mem {
                        self.state_mem
                            .dependants
                            .push((MemberId(*current_id), self.state_mem.dep_kind.clone()));
                    }
                }
                *current_id += 1;
                for d in &mut self.depandants {
                    self.state_mem
                        .dependants
                        .push((MemberId(*current_id), d.state_mem.dep_kind.clone()));
                    d.set_ids(current_id);
                }
            }

            fn contains_member(&self, member: &Member) -> bool {
                if self.state_mem.mem == member {
                    true
                } else {
                    self.depandants.iter().any(|d| d.contains_member(member))
                }
            }

            // check if there are any mixed child/parent dependancies
            fn check(&self) -> Option<Error> {
                self.kind().err()
            }

            fn kind(&self) -> Result<&DepKind> {
                fn reduce_kind<'a>(dk1: &'a DepKind, dk2: &'a DepKind) -> Result<&'a DepKind> {
                    match (dk1, dk2) {
                        (DepKind::Child, DepKind::Parent) | (DepKind::Parent, DepKind::Child) => {
                            Err(Error::new(
                                Span::call_site(),
                                "There is a ChildDepState that depends on a ParentDepState",
                            ))
                        }
                        // node dep state takes the lowest priority
                        (DepKind::Node, important) | (important, DepKind::Node) => Ok(important),
                        // they are the same
                        (fst, _) => Ok(fst),
                    }
                }
                reduce_kind(
                    self.depandants
                        .iter()
                        .try_fold(&DepKind::Node, |dk1, dk2| reduce_kind(dk1, dk2.kind()?))?,
                    &self.state_mem.dep_kind,
                )
            }

            fn insert_dependant(&mut self, other: DepNode<'a>) -> bool {
                let dep = other.state_mem.dep_mem.unwrap();
                if self.contains_member(dep) {
                    if self.state_mem.mem == dep {
                        self.depandants.push(other);
                        true
                    } else {
                        self.depandants
                            .iter_mut()
                            .find(|d| d.contains_member(dep))
                            .unwrap()
                            .insert_dependant(other)
                    }
                } else {
                    false
                }
            }
        }

        // members need to be sorted so that members are updated after the members they depend on
        let mut roots: Vec<DepNode> = vec![];
        for m in state_members {
            if let Some(dep) = m.dep_mem {
                let root_depends_on = roots
                    .iter()
                    .filter_map(|m| m.state_mem.dep_mem)
                    .any(|d| m.mem == d);

                if let Some(r) = roots.iter_mut().find(|r| r.contains_member(dep)) {
                    let new = DepNode::new(m);
                    if root_depends_on {
                        return Err(Error::new(
                            new.state_mem.mem.ident.span(),
                            format!("{} has a circular dependancy", new.state_mem.mem.ident),
                        ));
                    }
                    // return Err(Error::new(new.state_mem.mem.ident.span(), "stuff"));
                    r.insert_dependant(new);
                    continue;
                }
            }
            let mut new = DepNode::new(m);
            let mut i = 0;
            while i < roots.len() {
                if roots[i].state_mem.dep_mem == Some(new.state_mem.mem) {
                    let child = roots.remove(i);
                    new.insert_dependant(child);
                } else {
                    i += 1;
                }
            }
            roots.push(new);
        }
        parse_err?;
        let mut current_id = 0;
        for r in &mut roots {
            r.set_ids(&mut current_id);
        }
        if let Some(err) = roots.iter().find_map(DepNode::check) {
            Err(err)
        } else {
            let state_members: Vec<_> = roots
                .into_iter()
                .flat_map(|r| r.flatten().into_iter())
                .collect();

            Ok(Self {
                state_members,
                child_states: child_states.collect(),
            })
        }
    }
}

struct Dependancy {
    ctx_ty: Option<Type>,
    dep: Option<Ident>,
}

impl Parse for Dependancy {
    fn parse(input: ParseStream) -> Result<Self> {
        let dep = input
            .parse()
            .ok()
            .filter(|i: &Ident| format!("{}", i) != "NONE");
        let comma: Option<Token![,]> = input.parse().ok();
        let ctx_ty = input.parse().ok();
        Ok(Self {
            ctx_ty: comma.and(ctx_ty),
            dep,
        })
    }
}

#[derive(PartialEq, Debug)]
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

#[derive(Debug, Clone)]
struct StateMember<'a> {
    mem: &'a Member,
    dep_kind: DepKind,
    dep_mem: Option<&'a Member>,
    ctx_ty: Option<Type>,
    dependants: Vec<(dioxus_native_core::state::MemberId, DepKind)>,
    // This is just the index of the final order of the struct it is used to communicate which parts need updated and what order to update them in.
    member_id: dioxus_native_core::state::MemberId,
}

impl<'a> StateMember<'a> {
    fn parse(
        field: &Field,
        mem: &'a Member,
        parent: &'a Struct,
    ) -> Result<Option<StateMember<'a>>> {
        let mut err = Ok(());
        let member = field.attrs.iter().find_map(|a| {
            let dep_kind = a
                .path
                .get_ident()
                .and_then(|i| match i.to_string().as_str() {
                    "node_dep_state" => Some(DepKind::Node),
                    "child_dep_state" => Some(DepKind::Child),
                    "parent_dep_state" => Some(DepKind::Parent),
                    _ => None,
                })?;
            match a.parse_args::<Dependancy>() {
                Ok(dependancy) => {
                    let dep_mem = if let Some(name) = &dependancy.dep {
                        if let Some(found) = parent.members.iter().find(|m| &m.ident == name) {
                            Some(found)
                        } else {
                            err = Err(Error::new(
                                name.span(),
                                format!("{} not found in {}", name, parent.name),
                            ));
                            None
                        }
                    } else {
                        None
                    };
                    Some(Self {
                        mem,
                        dep_kind,
                        dep_mem,
                        ctx_ty: dependancy.ctx_ty,
                        dependants: Vec::new(),
                        member_id: dioxus_native_core::state::MemberId(0),
                    })
                }
                Err(e) => {
                    err = Err(e);
                    None
                }
            }
        });
        err?;
        Ok(member)
    }

    fn reduce_self(&self) -> quote::__private::TokenStream {
        let ident = &self.mem.ident;
        let get_ctx = if let Some(ctx_ty) = &self.ctx_ty {
            if ctx_ty == &parse_quote!(()) {
                quote! {&()}
            } else {
                let msg = ctx_ty.to_token_stream().to_string() + " not found in context";
                quote! {ctx.get().expect(#msg)}
            }
        } else {
            quote! {&()}
        };
        let states_changed = {
            let child_dep = self
                .dependants
                .iter()
                .filter(|(_, kind)| kind == &DepKind::Child)
                .map(|(id, _)| id.0);
            let parent_dep = self
                .dependants
                .iter()
                .filter(|(_, kind)| kind == &DepKind::Parent)
                .map(|(id, _)| id.0);
            let node_dep = self
                .dependants
                .iter()
                .filter(|(_, kind)| kind == &DepKind::Node)
                .map(|(id, _)| id.0);
            match self.dep_kind {
                DepKind::Node => {
                    quote! {
                        dioxus_native_core::state::NodeStatesChanged{
                            node_dep: vec![#(dioxus_native_core::state::MemberId(#node_dep), )*],
                        }
                    }
                }
                DepKind::Child => {
                    quote! {
                        dioxus_native_core::state::ChildStatesChanged{
                            node_dep: vec![#(dioxus_native_core::state::MemberId(#node_dep), )*],
                            child_dep: vec![#(dioxus_native_core::state::MemberId(#child_dep), )*],
                        }
                    }
                }
                DepKind::Parent => {
                    quote! {
                        dioxus_native_core::state::ParentStatesChanged{
                            node_dep: vec![#(dioxus_native_core::state::MemberId(#node_dep), )*],
                            parent_dep: vec![#(dioxus_native_core::state::MemberId(#parent_dep), )*],
                        }
                    }
                }
            }
        };

        let ty = &self.mem.ty;
        let node_view =
            quote!(dioxus_native_core::node_ref::NodeView::new(node, #ty::NODE_MASK, vdom));
        if let Some(dep_ident) = &self.dep_mem.map(|m| &m.ident) {
            match self.dep_kind {
                DepKind::Node => {
                    quote!({
                        if self.#ident.reduce(#node_view, &self.#dep_ident, #get_ctx){
                            Some(#states_changed)
                        } else{
                            None
                        }
                    })
                }
                DepKind::Child => {
                    quote!({
                        if self.#ident.reduce(#node_view, children.iter().map(|s| &s.#dep_ident), #get_ctx){
                            Some(#states_changed)
                        } else{
                            None
                        }
                    })
                }
                DepKind::Parent => {
                    quote!({
                        if self.#ident.reduce(#node_view, parent.as_ref().map(|p| &p.#dep_ident), #get_ctx){
                            Some(#states_changed)
                        } else{
                            None
                        }
                    })
                }
            }
        } else {
            match self.dep_kind {
                DepKind::Node => {
                    quote!({
                        if self.#ident.reduce(#node_view, &(), #get_ctx){
                            Some(#states_changed)
                        } else{
                            None
                        }
                    })
                }
                DepKind::Child => {
                    quote!({
                        if self.#ident.reduce(#node_view, std::iter::empty(), #get_ctx){
                            Some(#states_changed)
                        } else{
                            None
                        }
                    })
                }
                DepKind::Parent => {
                    quote!({
                        if self.#ident.reduce(#node_view, Some(&()), #get_ctx){
                            Some(#states_changed)
                        } else{
                            None
                        }
                    })
                }
            }
        }
    }
}
