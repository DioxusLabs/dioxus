use dioxus_native_core::real_dom_new_api::*;
use dioxus_native_core_macro::*;

#[derive(State)]
struct Z {
    // depends on just attributes and no context
    #[node_dep_state()]
    x: A,
    // depends on attributes, the B component of children and i32 context
    #[child_dep_state(i32, B)]
    y: B,
    // depends on attributes, the C component of it's parent and a u8 context
    #[parent_dep_state(u8, C)]
    z: C,
}

// struct Z {
//     x: A,
//     y: B,
//     z: C,
// }

use dioxus_native_core::real_dom_new_api::NodeDepState;

#[derive(PartialEq)]
struct A;
impl NodeDepState for A {
    type Ctx = ();
    fn reduce(&mut self, _: NodeRef, _: &()) {
        todo!()
    }
}

#[derive(PartialEq)]
struct B;
impl ChildDepState for B {
    type Ctx = i32;
    type DepState = Self;
    fn reduce(
        &mut self,
        _: dioxus_native_core::real_dom_new_api::NodeRef,
        _: Vec<&Self::DepState>,
        _: &i32,
    ) {
        todo!()
    }
}

#[derive(PartialEq)]
struct C;
impl ParentDepState for C {
    type Ctx = u8;
    type DepState = Self;
    fn reduce(
        &mut self,
        _: dioxus_native_core::real_dom_new_api::NodeRef,
        _: &Self::DepState,
        _: &u8,
    ) {
        todo!()
    }
}
