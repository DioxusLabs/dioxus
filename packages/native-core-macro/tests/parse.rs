use dioxus_native_core::node_ref::*;
use dioxus_native_core::state::*;
use dioxus_native_core_macro::*;

#[derive(State, Default, Clone)]
#[allow(dead_code)]
struct Z {
    // depends on text, the C component of it's parent and a u16 context
    #[parent_dep_state(c, u16)]
    d: D,
    // depends on just attributes and no context
    #[node_dep_state()]
    a: A,
    // depends on the B component of children and i32 context
    #[child_dep_state(b, i32)]
    b: B,
    // depends on the C component of it's parent and a u8 context
    #[parent_dep_state(c, u8)]
    c: C,
    // this will remain uneffected on updates
    n: i32,
}

use dioxus_native_core::state::NodeDepState;

#[derive(Default, Clone, Debug)]
struct A;
impl NodeDepState for A {
    type Ctx = ();
    type DepState = ();
    const NODE_MASK: NodeMask = NodeMask::new(AttributeMask::All, false, false, false);
    fn reduce(&mut self, _: NodeView, _: &Self::DepState, _: &()) -> bool {
        todo!()
    }
}

#[derive(Default, Clone, Debug)]
struct B;
impl ChildDepState for B {
    type Ctx = i32;
    type DepState = Self;
    fn reduce<'a>(
        &mut self,
        _: NodeView,
        _: impl Iterator<Item = &'a Self::DepState>,
        _: &i32,
    ) -> bool
    where
        Self::DepState: 'a,
    {
        todo!()
    }
}

#[derive(Default, Clone, Debug)]
struct C;
impl ParentDepState for C {
    type Ctx = u8;
    type DepState = Self;
    fn reduce(&mut self, _: NodeView, _: Option<&Self::DepState>, _: &u8) -> bool {
        todo!()
    }
}

#[derive(Default, Clone, Debug)]
struct D;
impl ParentDepState for D {
    type Ctx = u16;
    type DepState = C;
    const NODE_MASK: NodeMask = NodeMask::new(AttributeMask::NONE, false, false, true);
    fn reduce(&mut self, _: NodeView, _: Option<&Self::DepState>, _: &u16) -> bool {
        todo!()
    }
}
