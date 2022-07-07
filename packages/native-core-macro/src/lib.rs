extern crate proc_macro;

mod sorted_slice;

use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use sorted_slice::StrSlice;
use syn::parenthesized;
use syn::parse::ParseBuffer;
use syn::punctuated::Punctuated;
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
            let members: Vec<_> = state_strct
                .state_members
                .iter()
                .map(|m| &m.mem.ident)
                .collect();
            let member_types = state_strct.state_members.iter().map(|m| &m.mem.ty);
            let resolve_members = state_strct
                .state_members
                .iter()
                .map(|m| state_strct.resolve(m));

            let child_types = state_strct.child_states.iter().map(|s| &s.ty);
            let child_members = state_strct.child_states.iter().map(|s| &s.ident);

            let gen = quote! {
                impl State for #type_name {
                    fn update<'a, T: dioxus_native_core::traversable::Traversable<Node = Self, Id = dioxus_core::ElementId>>(
                        dirty: &[(dioxus_core::ElementId, dioxus_native_core::node_ref::NodeMask)],
                        state_tree: &'a mut T,
                        vdom: &'a dioxus_core::VirtualDom,
                        ctx: &anymap::AnyMap,
                    ) -> fxhash::FxHashSet<dioxus_core::ElementId>{
                        #[derive(Eq, PartialEq)]
                        struct HeightOrdering {
                            height: u16,
                            id: dioxus_core::ElementId,
                        }

                        impl HeightOrdering {
                            fn new(height: u16, id: dioxus_core::ElementId) -> Self {
                                HeightOrdering {
                                    height,
                                    id,
                                }
                            }
                        }

                        impl Ord for HeightOrdering {
                            fn cmp(&self, other: &Self) -> std::cmp::Ordering {
                                self.height.cmp(&other.height).then(self.id.0.cmp(&other.id.0))
                            }
                        }

                        impl PartialOrd for HeightOrdering {
                            fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
                                Some(self.cmp(&other))
                            }
                        }

                        struct MembersDirty {
                            #(#members: bool, )*
                        }

                        impl MembersDirty {
                            fn new() -> Self {
                                Self {#(#members: false),*}
                            }

                            fn any(&self) -> bool {
                                #(self.#members || )* false
                            }
                        }

                        let mut dirty_elements = fxhash::FxHashSet::default();
                        // the states of any elements that are dirty
                        let mut states = fxhash::FxHashMap::default();

                        for (id, mask) in dirty {
                            let members_dirty = MembersDirty {
                                #(#members: #member_types::NODE_MASK.overlaps(mask),)*
                            };
                            if members_dirty.any(){
                                states.insert(*id, members_dirty);
                            }
                            dirty_elements.insert(*id);
                        }

                        #(
                            #resolve_members;
                        )*

                        #(
                            dirty_elements.extend(
                                <#child_types as dioxus_native_core::state::State>::update(
                                    dirty,
                                    &mut state_tree.map(|n| &n.#child_members, |n| &mut n.#child_members),
                                    vdom,
                                    ctx,
                                )
                            );
                        )*

                        dirty_elements
                    }
                }
            };
            gen.into()
        }
        Err(e) => e.into_compile_error().into(),
    }
}

struct Depenadants<'a> {
    node: Vec<&'a Member>,
    child: Vec<&'a Member>,
    parent: Vec<&'a Member>,
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
        let mut unordered_state_members: Vec<_> = strct
            .members
            .iter()
            .zip(fields.iter())
            .filter_map(|(m, f)| match StateMember::parse(f, m, strct) {
                Ok(m) => m,
                Err(err) => {
                    parse_err = Err(err);
                    None
                }
            })
            .collect();
        parse_err?;

        let mut state_members = Vec::new();
        while !unordered_state_members.is_empty() {
            let mut resolved = false;
            for i in 0..unordered_state_members.len() {
                let mem = &mut unordered_state_members[i];
                if mem.dep_mems.iter().all(|(dep, resolved)| {
                    *resolved || (*dep == mem.mem && mem.dep_kind != DepKind::Node)
                }) {
                    let mem = unordered_state_members.remove(i);
                    // mark any dependancy that depends on this member as resolved
                    for member in unordered_state_members.iter_mut() {
                        for (dep, resolved) in &mut member.dep_mems {
                            *resolved |= *dep == mem.mem;
                        }
                    }
                    state_members.push(mem);
                    resolved = true;
                    break;
                }
            }
            if !resolved {
                return Err(Error::new(
                    strct.name.span(),
                    format!(
                        "{} has circular dependacy in {:?}",
                        strct.name,
                        unordered_state_members
                            .iter()
                            .map(|m| format!("{}", &m.mem.ident))
                            .collect::<Vec<_>>()
                    ),
                ));
            }
        }

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

        // members need to be sorted so that members are updated after the members they depend on
        Ok(Self {
            state_members,
            child_states: child_states.collect(),
        })
    }

    fn get_depenadants(&self, mem: &Member) -> Depenadants {
        let mut dependants = Depenadants {
            node: Vec::new(),
            child: Vec::new(),
            parent: Vec::new(),
        };
        for member in &self.state_members {
            for (dep, _) in &member.dep_mems {
                if *dep == mem {
                    match member.dep_kind {
                        DepKind::Node => dependants.node.push(member.mem),
                        DepKind::Child => dependants.child.push(member.mem),
                        DepKind::Parent => dependants.parent.push(member.mem),
                    }
                }
            }
        }
        dependants
    }

    fn update_dependants(&self, mem: &Member) -> impl ToTokens {
        let dep = self.get_depenadants(mem);
        let update_child_dependants = if dep.child.is_empty() {
            quote!()
        } else {
            let insert = dep.child.iter().map(|d|{
                if *d == mem {
                    quote! {
                        let seeking = HeightOrdering::new(state_tree.height(parent_id).unwrap(), parent_id);
                        if let Err(idx) = resolution_order
                            .binary_search_by(|ordering| ordering.cmp(&seeking).reverse()){
                            resolution_order.insert(
                                idx,
                                seeking,
                            );
                        }
                    }
                } else {
                    quote! {}
                }
            });
            let update: Vec<_> = dep
                .child
                .iter()
                .map(|d| {
                    let ident = &d.ident;
                    quote! {
                        dirty.#ident = true;
                    }
                })
                .collect();
            quote! {
                if let Some(parent_id) = state_tree.parent(id) {
                    #(#insert)*
                    if let Some(dirty) = states.get_mut(&parent_id) {
                        #(#update)*
                    }
                    else {
                        let mut dirty = MembersDirty::new();
                        #(#update)*
                        states.insert(parent_id, dirty);
                    }
                }
            }
        };
        let node_dependants: Vec<_> = dep.node.iter().map(|d| &d.ident).collect();
        let update_node_dependants = quote! {#(members_dirty.#node_dependants = true;)*};
        let update_parent_dependants = if dep.parent.is_empty() {
            quote!()
        } else {
            let insert = dep.parent.iter().map(|d| {
                if *d == mem {
                    quote! {
                        let seeking = HeightOrdering::new(state_tree.height(*child_id).unwrap(), *child_id);
                        if let Err(idx) = resolution_order
                            .binary_search(&seeking){
                            resolution_order.insert(
                                idx,
                                seeking,
                            );
                        }
                    }
                } else {
                    quote! {}
                }
            });
            let update: Vec<_> = dep
                .parent
                .iter()
                .map(|d| {
                    let ident = &d.ident;
                    quote! {
                        dirty.#ident = true;
                    }
                })
                .collect();
            quote! {
                for child_id in state_tree.children(id) {
                    #(#insert)*
                    if let Some(dirty) = states.get_mut(&child_id) {
                        #(#update)*
                    }
                    else {
                        let mut dirty = MembersDirty::new();
                        #(#update)*
                        states.insert(*child_id, dirty);
                    }
                }
            }
        };

        quote! {
            #update_node_dependants
            #update_child_dependants
            #update_parent_dependants
        }
    }

    fn resolve(&self, mem: &StateMember) -> impl ToTokens {
        let reduce_member = mem.reduce_self();
        let update_dependant = self.update_dependants(mem.mem);
        let member = &mem.mem.ident;

        match mem.dep_kind {
            DepKind::Parent => {
                quote! {
                    // resolve parent dependant state
                    let mut resolution_order = states.keys().copied().map(|id| HeightOrdering::new(state_tree.height(id).unwrap(), id)).collect::<Vec<_>>();
                    resolution_order.sort();
                    let mut i = 0;
                    while i < resolution_order.len(){
                        let id = resolution_order[i].id;
                        let vnode = vdom.get_element(id).unwrap();
                        let members_dirty = states.get_mut(&id).unwrap();
                        let (current_state, parent) = state_tree.get_node_parent_mut(id);
                        let current_state = current_state.unwrap();
                        if members_dirty.#member && #reduce_member {
                            dirty_elements.insert(id);
                            #update_dependant
                        }
                        i += 1;
                    }
                }
            }
            DepKind::Child => {
                quote! {
                    // resolve child dependant state
                    let mut resolution_order = states.keys().copied().map(|id| HeightOrdering::new(state_tree.height(id).unwrap(), id)).collect::<Vec<_>>();
                    resolution_order.sort_by(|height_ordering1, height_ordering2| {
                        height_ordering1.cmp(&height_ordering2).reverse()
                    });
                    let mut i = 0;
                    while i < resolution_order.len(){
                        let id = resolution_order[i].id;
                        let vnode = vdom.get_element(id).unwrap();
                        let members_dirty = states.get_mut(&id).unwrap();
                        let (current_state, children) = state_tree.get_node_children_mut(id);
                        let current_state = current_state.unwrap();
                        if members_dirty.#member && #reduce_member {
                            dirty_elements.insert(id);
                            #update_dependant
                        }
                        i += 1;
                    }
                }
            }
            DepKind::Node => {
                quote! {
                    // resolve node dependant state
                    let mut resolution_order = states.keys().copied().collect::<Vec<_>>();
                    let mut i = 0;
                    while i < resolution_order.len(){
                        let id = resolution_order[i];
                        let vnode = vdom.get_element(id).unwrap();
                        let members_dirty = states.get_mut(&id).unwrap();
                        let current_state = state_tree.get_mut(id).unwrap();
                        if members_dirty.#member && #reduce_member {
                            dirty_elements.insert(id);
                            #update_dependant
                        }
                        i += 1;
                    }
                }
            }
        }
    }
}

fn try_parenthesized(input: ParseStream) -> Result<ParseBuffer> {
    let inside;
    parenthesized!(inside in input);
    Ok(inside)
}

struct Dependancy {
    ctx_ty: Option<Type>,
    deps: Vec<Ident>,
}

impl Parse for Dependancy {
    fn parse(input: ParseStream) -> Result<Self> {
        let deps: Option<Punctuated<Ident, Token![,]>> = {
            try_parenthesized(input)
                .ok()
                .and_then(|inside| inside.parse_terminated(Ident::parse).ok())
        };
        let deps: Vec<_> = deps
            .map(|deps| deps.into_iter().collect())
            .or_else(|| {
                input
                    .parse::<Ident>()
                    .ok()
                    .filter(|i: &Ident| format!("{}", i) != "NONE")
                    .map(|i| vec![i])
            })
            .unwrap_or_default();
        let comma: Option<Token![,]> = input.parse().ok();
        let ctx_ty = input.parse().ok();
        Ok(Self {
            ctx_ty: comma.and(ctx_ty),
            deps,
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
    // the depenancy and if it is satified
    dep_mems: Vec<(&'a Member, bool)>,
    ctx_ty: Option<Type>,
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
                    let dep_mems = dependancy
                        .deps
                        .iter()
                        .filter_map(|name| {
                            if let Some(found) = parent.members.iter().find(|m| &m.ident == name) {
                                Some((found, false))
                            } else {
                                err = Err(Error::new(
                                    name.span(),
                                    format!("{} not found in {}", name, parent.name),
                                ));
                                None
                            }
                        })
                        .collect();
                    Some(Self {
                        mem,
                        dep_kind,
                        dep_mems,
                        ctx_ty: dependancy.ctx_ty,
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

        let ty = &self.mem.ty;
        let node_view =
            quote!(dioxus_native_core::node_ref::NodeView::new(vnode, #ty::NODE_MASK, vdom));
        let dep_idents = self.dep_mems.iter().map(|m| &m.0.ident);
        match self.dep_kind {
            DepKind::Node => {
                quote!({
                    current_state.#ident.reduce(#node_view, (#(&current_state.#dep_idents,)*), #get_ctx)
                })
            }
            DepKind::Child => {
                quote!({
                    current_state.#ident.reduce(#node_view, children.iter().map(|c| (#(&c.#dep_idents)*)), #get_ctx)
                })
            }
            DepKind::Parent => {
                quote!({
                    current_state.#ident.reduce(#node_view, parent.as_ref().map(|p| (#(&p.#dep_idents)*)), #get_ctx)
                })
            }
        }
    }
}
