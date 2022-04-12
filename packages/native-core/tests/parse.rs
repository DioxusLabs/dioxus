use dioxus_native_core::real_dom_new_api::*;
use dioxus_native_core_macro::*;

#[derive(State, Default, Clone)]
struct Z {
    // depends on just attributes and no context
    #[node_dep_state()]
    x: A,
    // depends on attributes, the B component of children and i32 context
    #[child_dep_state(B, i32)]
    y: B,
    // depends on attributes, the C component of it's parent and a u8 context
    #[parent_dep_state(C, u8)]
    z: C,
}

// struct Z {
//     x: A,
//     y: B,
//     z: C,
// }

use dioxus_native_core::real_dom_new_api::NodeDepState;

#[derive(Default, Clone)]
struct A;
impl NodeDepState for A {
    type Ctx = ();
    fn reduce(&mut self, _: NodeView, _: &()) -> bool {
        todo!()
    }
}

#[derive(Default, Clone)]
struct B;
impl ChildDepState for B {
    type Ctx = i32;
    type DepState = Self;
    fn reduce(
        &mut self,
        _: dioxus_native_core::real_dom_new_api::NodeView,
        _: Vec<&Self::DepState>,
        _: &i32,
    ) -> bool {
        todo!()
    }
}

#[derive(Default, Clone)]
struct C;
impl ParentDepState for C {
    type Ctx = u8;
    type DepState = Self;
    fn reduce(
        &mut self,
        _: dioxus_native_core::real_dom_new_api::NodeView,
        _: Option<&Self::DepState>,
        _: &u8,
    ) -> bool {
        todo!()
    }
}
