extern crate proc_macro;

mod sorted_slice;

use proc_macro::TokenStream;
use quote::{quote, ToTokens, __private::Span};
use sorted_slice::StrSlice;
use syn::parenthesized;
use syn::parse::ParseBuffer;
use syn::punctuated::Punctuated;
use syn::{
    self,
    parse::{Parse, ParseStream, Result},
    parse_macro_input, parse_quote, Error, Field, Ident, Token, Type,
};

/// Sorts a slice of string literals at compile time.
#[proc_macro]
pub fn sorted_str_slice(input: TokenStream) -> TokenStream {
    let slice: StrSlice = parse_macro_input!(input as StrSlice);
    let strings = slice.map.values();
    quote!([#(#strings, )*]).into()
}

#[derive(PartialEq, Debug, Clone)]
enum DependencyKind {
    Node,
    Child,
    Parent,
}

/// Derive's the state from any elements that have a node_dep_state, child_dep_state, parent_dep_state, or state attribute.
///
/// # Declaring elements
/// Each of the attributes require specifying the members of the struct it depends on to allow the macro to find the optimal resultion order.
/// These dependencies should match the types declared in the trait the member implements.
///
/// # The node_dep_state attribute
/// The node_dep_state attribute declares a member that implements the NodeDepState trait.
/// ```rust, ignore
/// #[derive(State)]
/// struct MyStruct {
///     // MyDependency implements ChildDepState<()>
///     #[node_dep_state()]
///     my_dependency_1: MyDependency,
///     // MyDependency2 implements ChildDepState<(MyDependency,)>
///     #[node_dep_state(my_dependency_1)]
///     my_dependency_2: MyDependency2,
/// }
/// // or
/// #[derive(State)]
/// struct MyStruct {
///     // MyDependnancy implements NodeDepState<()>
///     #[node_dep_state()]
///     my_dependency_1: MyDependency,
///     // MyDependency2 implements NodeDepState<()>
///     #[node_dep_state()]
///     my_dependency_2: MyDependency2,
///     // MyDependency3 implements NodeDepState<(MyDependency, MyDependency2)> with Ctx = f32
///     #[node_dep_state((my_dependency_1, my_dependency_2), f32)]
///     my_dependency_3: MyDependency2,
/// }
/// ```
/// # The child_dep_state attribute
/// The child_dep_state attribute declares a member that implements the ChildDepState trait.
/// ```rust, ignore
/// #[derive(State)]
/// struct MyStruct {
///     // MyDependnacy implements ChildDepState with DepState = Self
///     #[child_dep_state(my_dependency_1)]
///     my_dependency_1: MyDependency,
/// }
/// // or
/// #[derive(State)]
/// struct MyStruct {
///     // MyDependnacy implements ChildDepState with DepState = Self
///     #[child_dep_state(my_dependency_1)]
///     my_dependency_1: MyDependency,
///     // MyDependnacy2 implements ChildDepState with DepState = MyDependency and Ctx = f32
///     #[child_dep_state(my_dependency_1, f32)]
///     my_dependency_2: MyDependency2,
/// }
/// ```
/// # The parent_dep_state attribute
/// The parent_dep_state attribute declares a member that implements the ParentDepState trait.
/// The parent_dep_state attribute can be called in the forms:
/// ```rust, ignore
/// #[derive(State)]
/// struct MyStruct {
///     // MyDependnacy implements ParentDepState with DepState = Self
///     #[parent_dep_state(my_dependency_1)]
///     my_dependency_1: MyDependency,
/// }
/// // or
/// #[derive(State)]
/// struct MyStruct {
///     // MyDependnacy implements ParentDepState with DepState = Self
///     #[parent_dep_state(my_dependency_1)]
///     my_dependency_1: MyDependency,
///     // MyDependnacy2 implements ParentDepState with DepState = MyDependency and Ctx = f32
///     #[parent_dep_state(my_dependency_1, f32)]
///     my_dependency_2: MyDependency2,
/// }
/// ```
///
/// # Combining dependancies
/// The node_dep_state, parent_dep_state, and child_dep_state attributes can be combined to allow for more complex dependancies.
/// For example if we wanted to combine the font that is passed from the parent to the child and the layout of the size children to find the size of the current node we could do:
/// ```rust, ignore
/// #[derive(State)]
/// struct MyStruct {
///     // ChildrenSize implements ChildDepState with DepState = Size
///     #[child_dep_state(size)]
///     children_size: ChildrenSize,
///     // FontSize implements ParentDepState with DepState = Self
///     #[parent_dep_state(font_size)]
///     font_size: FontSize,
///     // Size implements NodeDepState<(ChildrenSize, FontSize)>
///     #[parent_dep_state((children_size, font_size))]
///     size: Size,
/// }
/// ```
///
/// # The state attribute
/// The state macro declares a member that implements the State trait. This allows you to organize your state into multiple isolated components.
/// Unlike the other attributes, the state attribute does not accept any arguments, because a nested state cannot depend on any other part of the state.
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
            let passes = state_strct.state_members.iter().map(|m| {
                let unit = &m.mem.unit_type;
                match m.dep_kind {
                    DependencyKind::Node => quote! {dioxus_native_core::AnyPass::Node(&#unit)},
                    DependencyKind::Child => quote! {dioxus_native_core::AnyPass::Upward(&#unit)},
                    DependencyKind::Parent => {
                        quote! {dioxus_native_core::AnyPass::Downward(&#unit)}
                    }
                }
            });
            let member_types = state_strct.state_members.iter().map(|m| &m.mem.ty);
            let impl_members = state_strct
                .state_members
                .iter()
                .map(|m| m.impl_pass(state_strct.ty));

            let gen = quote! {
                #(#impl_members)*
                impl State for #type_name {
                    const PASSES: &'static [dioxus_native_core::AnyPass<dioxus_native_core::node::Node<Self>>] = &[
                        #(#passes),*
                    ];
                    const MASKS: &'static [dioxus_native_core::NodeMask] = &[#(#member_types::NODE_MASK),*];
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
        let members = fields
            .iter()
            .enumerate()
            .filter_map(|(i, f)| Member::parse(f, i as u64))
            .collect();
        Self { name, members }
    }
}

struct StateStruct<'a> {
    state_members: Vec<StateMember<'a>>,
    #[allow(unused)]
    child_states: Vec<&'a Member>,
    ty: &'a Ident,
}

impl<'a> StateStruct<'a> {
    /// Parse the state structure, and find a resolution order that will allow us to update the state for each node in after the state(s) it depends on have been resolved.
    fn parse(fields: &[&'a Field], strct: &'a Struct) -> Result<Self> {
        let mut parse_err = Ok(());
        let mut state_members: Vec<_> = strct
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
        for i in 0..state_members.len() {
            let deps: Vec<_> = state_members[i].dep_mems.iter().map(|m| m.id).collect();
            for dep in deps {
                state_members[dep as usize].dependant_mems.push(i as u64);
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
            ty: &strct.name,
            state_members,
            child_states: child_states.collect(),
        })
    }
}

fn try_parenthesized(input: ParseStream) -> Result<ParseBuffer> {
    let inside;
    parenthesized!(inside in input);
    Ok(inside)
}

struct Dependency {
    ctx_ty: Option<Type>,
    deps: Vec<Ident>,
}

impl Parse for Dependency {
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

/// The type of the member and the ident of the member
#[derive(PartialEq, Debug)]
struct Member {
    id: u64,
    ty: Type,
    unit_type: Ident,
    ident: Ident,
}

impl Member {
    fn parse(field: &Field, id: u64) -> Option<Self> {
        Some(Self {
            id,
            ty: field.ty.clone(),
            unit_type: Ident::new(
                ("_Unit".to_string() + field.ty.to_token_stream().to_string().as_str()).as_str(),
                Span::call_site(),
            )
            .into(),
            ident: field.ident.as_ref()?.clone(),
        })
    }
}

#[derive(Debug, Clone)]
struct StateMember<'a> {
    mem: &'a Member,
    // the kind of dependncies this state has
    dep_kind: DependencyKind,
    // the depenancy and if it is satified
    dep_mems: Vec<&'a Member>,
    // any members that depend on this member
    dependant_mems: Vec<u64>,
    // the context this state requires
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
                    "node_dep_state" => Some(DependencyKind::Node),
                    "child_dep_state" => Some(DependencyKind::Child),
                    "parent_dep_state" => Some(DependencyKind::Parent),
                    _ => None,
                })?;
            match a.parse_args::<Dependency>() {
                Ok(dependency) => {
                    let dep_mems = dependency
                        .deps
                        .iter()
                        .filter_map(|name| {
                            if let Some(found) = parent.members.iter().find(|m| &m.ident == name) {
                                Some(found)
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
                        dependant_mems: Vec::new(),
                        ctx_ty: dependency.ctx_ty,
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

    /// generate code to call the resolve function for the state. This does not handle checking if resolving the state is necessary, or marking the states that depend on this state as dirty.
    fn impl_pass(&self, parent_type: &Ident) -> quote::__private::TokenStream {
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
        let unit_type = &self.mem.unit_type;
        let node_view = quote!(dioxus_native_core::node_ref::NodeView::new(unsafe{&*{&node.node_data as *const _}}, #ty::NODE_MASK));
        let dep_idents = self.dep_mems.iter().map(|m| &m.ident);
        let impl_specific = match self.dep_kind {
            DependencyKind::Node => {
                quote! {
                    impl dioxus_native_core::NodePass<dioxus_native_core::node::Node<#parent_type>> for #unit_type {
                        fn pass(&self, node: &mut dioxus_native_core::node::Node<#parent_type>, ctx: &dioxus_native_core::SendAnyMap) -> bool {
                            node.state.#ident.reduce(#node_view, (#(&node.state.#dep_idents,)*), #get_ctx)
                        }
                    }
                }
            }
            DependencyKind::Child => {
                let update = if self.dep_mems.iter().any(|m| m.id == self.mem.id) {
                    quote! {
                        if update {
                            dioxus_native_core::PassReturn{
                                progress: true,
                                mark_dirty: true,
                            }
                        } else {
                            dioxus_native_core::PassReturn{
                                progress: false,
                                mark_dirty: false,
                            }
                        }
                    }
                } else {
                    quote! {
                        if update {
                            dioxus_native_core::PassReturn{
                                progress: false,
                                mark_dirty: true,
                            }
                        } else {
                            dioxus_native_core::PassReturn{
                                progress: false,
                                mark_dirty: false,
                            }
                        }
                    }
                };
                quote!(
                    impl dioxus_native_core::UpwardPass<dioxus_native_core::node::Node<#parent_type>> for #unit_type{
                        fn pass<'a>(
                            &self,
                            node: &mut dioxus_native_core::node::Node<#parent_type>,
                            children: &mut dyn Iterator<Item = &'a mut dioxus_native_core::node::Node<#parent_type>>,
                            ctx: &dioxus_native_core::SendAnyMap,
                        ) -> dioxus_native_core::PassReturn {
                            let update = node.state.#ident.reduce(#node_view, children.map(|c| (#(&c.state.#dep_idents,)*)), #get_ctx);
                            #update
                        }
                    }
                )
            }
            DependencyKind::Parent => {
                let update = if self.dep_mems.iter().any(|m| m.id == self.mem.id) {
                    quote! {
                        if update {
                            dioxus_native_core::PassReturn{
                                progress: true,
                                mark_dirty: true,
                            }
                        } else {
                            dioxus_native_core::PassReturn{
                                progress: false,
                                mark_dirty: false,
                            }
                        }
                    }
                } else {
                    quote! {
                        if update {
                            dioxus_native_core::PassReturn{
                                progress: false,
                                mark_dirty: true,
                            }
                        } else {
                            dioxus_native_core::PassReturn{
                                progress: false,
                                mark_dirty: false,
                            }
                        }
                    }
                };
                quote!(
                    impl dioxus_native_core::DownwardPass<dioxus_native_core::node::Node<#parent_type>> for #unit_type {
                        fn pass(&self, node: &mut dioxus_native_core::node::Node<#parent_type>, parent: Option<&mut dioxus_native_core::node::Node<#parent_type>>, ctx: &dioxus_native_core::SendAnyMap) -> dioxus_native_core::PassReturn{
                            let update = node.state.#ident.reduce(#node_view, parent.as_ref().map(|p| (#(&p.state.#dep_idents,)*)), #get_ctx);
                            #update
                        }
                    }
                )
            }
        };
        let pass_id = self.mem.id;
        let depenancies = self.dep_mems.iter().map(|m| m.id);
        let dependants = &self.dependant_mems;
        let mask = self
            .dep_mems
            .iter()
            .map(|m| 1u64 << m.id)
            .fold(1 << self.mem.id, |a, b| a | b);
        quote! {
            #[derive(Clone, Copy)]
            struct #unit_type;
            #impl_specific
            impl dioxus_native_core::Pass for #unit_type {
                fn pass_id(&self) -> dioxus_native_core::PassId {
                    dioxus_native_core::PassId(#pass_id)
                }
                fn dependancies(&self) -> &'static [dioxus_native_core::PassId] {
                    &[#(dioxus_native_core::PassId(#depenancies)),*]
                }
                fn dependants(&self) -> &'static [dioxus_native_core::PassId] {
                    &[#(dioxus_native_core::PassId(#dependants)),*]
                }
                fn mask(&self) -> dioxus_native_core::MemberMask {
                    dioxus_native_core::MemberMask(#mask)
                }
            }
        }
    }
}
