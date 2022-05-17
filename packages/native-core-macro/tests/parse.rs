use dioxus_core::*;
use dioxus_native_core::node_ref::*;
use dioxus_native_core::state::{ChildDepState, NodeDepState, ParentDepState, State};
use dioxus_native_core_macro::State;

#[derive(Debug, Clone, Default, State)]
struct CallCounterState {
    #[child_dep_state(child_counter)]
    child_counter: ChildDepCallCounter,
    #[parent_dep_state(parent_counter)]
    parent_counter: ParentDepCallCounter,
    #[node_dep_state()]
    node_counter: NodeDepCallCounter,
}

#[derive(Debug, Clone, Default)]
struct ChildDepCallCounter(u32);
impl ChildDepState for ChildDepCallCounter {
    type Ctx = ();
    type DepState = Self;
    const NODE_MASK: NodeMask = NodeMask::ALL;
    fn reduce<'a>(
        &mut self,
        _: NodeView,
        _: impl Iterator<Item = &'a Self::DepState>,
        _: &Self::Ctx,
    ) -> bool
    where
        Self::DepState: 'a,
    {
        self.0 += 1;
        true
    }
}

#[derive(Debug, Clone, Default)]
struct ParentDepCallCounter(u32);
impl ParentDepState for ParentDepCallCounter {
    type Ctx = ();
    type DepState = Self;
    const NODE_MASK: NodeMask = NodeMask::ALL;
    fn reduce(
        &mut self,
        _node: NodeView,
        _parent: Option<&Self::DepState>,
        _ctx: &Self::Ctx,
    ) -> bool {
        self.0 += 1;
        true
    }
}

#[derive(Debug, Clone, Default)]
struct NodeDepCallCounter(u32);
impl NodeDepState for NodeDepCallCounter {
    type Ctx = ();
    type DepState = ();
    const NODE_MASK: NodeMask = NodeMask::ALL;
    fn reduce(&mut self, _node: NodeView, _sibling: Self::DepState, _ctx: &Self::Ctx) -> bool {
        self.0 += 1;
        true
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
struct BubbledUpStateTester(Option<String>, Vec<Box<BubbledUpStateTester>>);
impl ChildDepState for BubbledUpStateTester {
    type Ctx = u32;
    type DepState = Self;
    const NODE_MASK: NodeMask = NodeMask::new().with_tag();
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
