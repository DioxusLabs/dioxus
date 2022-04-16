use dioxus_native_core::node_ref::*;
use dioxus_native_core::state::{ChildDepState, NodeDepState, ParentDepState, State};
use dioxus_native_core_macro::State;
#[derive(Debug, Clone, PartialEq, Default)]
struct BubbledUpStateTester(Option<String>, Vec<Box<BubbledUpStateTester>>);
impl ChildDepState for BubbledUpStateTester {
    type Ctx = u32;
    type DepState = Self;
    const NODE_MASK: NodeMask = NodeMask::new(AttributeMask::NONE, true, false, false);
    fn reduce<'a>(
        &mut self,
        node: NodeView,
        children: impl Iterator<Item = &'a Self::DepState>,
        ctx: &Self::Ctx,
    ) -> bool
    where
        Self::DepState: 'a,
    {
        assert_eq!(*ctx, 42);
        *self = BubbledUpStateTester(
            node.tag().map(|s| s.to_string()),
            children.into_iter().map(|c| Box::new(c.clone())).collect(),
        );
        true
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
struct PushedDownStateTester(Option<String>, Option<Box<PushedDownStateTester>>);
impl ParentDepState for PushedDownStateTester {
    type Ctx = u32;
    type DepState = Self;
    const NODE_MASK: NodeMask = NodeMask::new(AttributeMask::NONE, true, false, false);
    fn reduce(&mut self, node: NodeView, parent: Option<&Self::DepState>, ctx: &Self::Ctx) -> bool {
        assert_eq!(*ctx, 42);
        *self = PushedDownStateTester(
            node.tag().map(|s| s.to_string()),
            parent.map(|c| Box::new(c.clone())),
        );
        true
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
struct NodeStateTester(Option<String>, Vec<(String, String)>);
impl NodeDepState for NodeStateTester {
    type Ctx = u32;
    type DepState = ();
    const NODE_MASK: NodeMask = NodeMask::new(AttributeMask::All, true, false, false);
    fn reduce(&mut self, node: NodeView, _sibling: &Self::DepState, ctx: &Self::Ctx) -> bool {
        assert_eq!(*ctx, 42);
        *self = NodeStateTester(
            node.tag().map(|s| s.to_string()),
            node.attributes()
                .map(|a| (a.name.to_string(), a.value.to_string()))
                .collect(),
        );
        true
    }
}

#[derive(State, Clone, Default, Debug)]
struct StateTester {
    #[child_dep_state(bubbled, u32)]
    bubbled: BubbledUpStateTester,
    #[parent_dep_state(pushed, u32)]
    pushed: PushedDownStateTester,
    #[node_dep_state(NONE, u32)]
    node: NodeStateTester,
}
