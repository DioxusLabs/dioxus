use dioxus::prelude::*;
use dioxus_native_core::real_dom::*;
use dioxus_native_core::state::{ChildDepState, NodeDepState, ParentDepState, State};
use dioxus_native_core::tree::TreeView;
use dioxus_native_core::{node_ref::*, NodeId, SendAnyMap};
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
    type DepState = (Self,);
    const NODE_MASK: NodeMask = NodeMask::ALL;
    fn reduce<'a>(
        &mut self,
        _: NodeView,
        _: impl Iterator<Item = (&'a Self,)>,
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
    type DepState = (Self,);
    const NODE_MASK: NodeMask = NodeMask::ALL;
    fn reduce(&mut self, _node: NodeView, _parent: Option<(&Self,)>, _ctx: &Self::Ctx) -> bool {
        self.0 += 1;
        println!("ParentDepCallCounter::reduce on {:?}\n{}", _node, self.0);
        true
    }
}

#[derive(Debug, Clone, Default)]
struct NodeDepCallCounter(u32);
impl NodeDepState for NodeDepCallCounter {
    type Ctx = ();
    type DepState = ();
    const NODE_MASK: NodeMask = NodeMask::ALL;
    fn reduce(&mut self, _node: NodeView, _sibling: (), _ctx: &Self::Ctx) -> bool {
        self.0 += 1;
        true
    }
}

#[allow(clippy::vec_box)]
#[derive(Debug, Clone, PartialEq, Default)]
struct BubbledUpStateTester(Option<String>, Vec<Box<BubbledUpStateTester>>);
impl ChildDepState for BubbledUpStateTester {
    type Ctx = u32;
    type DepState = (Self,);
    const NODE_MASK: NodeMask = NodeMask::new().with_tag();
    fn reduce<'a>(
        &mut self,
        node: NodeView,
        children: impl Iterator<Item = (&'a Self,)>,
        ctx: &Self::Ctx,
    ) -> bool
    where
        Self::DepState: 'a,
    {
        assert_eq!(*ctx, 42);
        *self = BubbledUpStateTester(
            node.tag().map(|s| s.to_string()),
            children
                .into_iter()
                .map(|(c,)| Box::new(c.clone()))
                .collect(),
        );
        true
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
struct PushedDownStateTester(Option<String>, Option<Box<PushedDownStateTester>>);
impl ParentDepState for PushedDownStateTester {
    type Ctx = u32;
    type DepState = (Self,);
    const NODE_MASK: NodeMask = NodeMask::new().with_tag();
    fn reduce(&mut self, node: NodeView, parent: Option<(&Self,)>, ctx: &Self::Ctx) -> bool {
        assert_eq!(*ctx, 42);
        *self = PushedDownStateTester(
            node.tag().map(|s| s.to_string()),
            parent.map(|(c,)| Box::new(c.clone())),
        );
        true
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
struct NodeStateTester(Option<String>, Vec<(String, String)>);
impl NodeDepState for NodeStateTester {
    type Ctx = u32;
    type DepState = ();
    const NODE_MASK: NodeMask = NodeMask::new_with_attrs(AttributeMask::All).with_tag();
    fn reduce(&mut self, node: NodeView, _sibling: (), ctx: &Self::Ctx) -> bool {
        assert_eq!(*ctx, 42);
        *self = NodeStateTester(
            node.tag().map(|s| s.to_string()),
            node.attributes()
                .map(|iter| {
                    iter.map(|a| {
                        (
                            a.attribute.name.to_string(),
                            a.value.as_text().unwrap().to_string(),
                        )
                    })
                    .collect()
                })
                .unwrap_or_default(),
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

#[test]
fn state_initial() {
    #[allow(non_snake_case)]
    fn Base(cx: Scope) -> Element {
        render! {
            div {
                p{
                    color: "red"
                }
                h1{}
            }
        }
    }

    let mut vdom = VirtualDom::new(Base);

    let mutations = vdom.rebuild();

    let mut dom: RealDom<StateTester> = RealDom::new();

    let (nodes_updated, _) = dom.apply_mutations(mutations);
    let mut ctx = SendAnyMap::new();
    ctx.insert(42u32);
    let _to_rerender = dom.update_state(nodes_updated, ctx);

    let root_div_id = dom.children_ids(NodeId(0)).unwrap()[0];
    let root_div = &dom.get(root_div_id).unwrap();
    assert_eq!(root_div.state.bubbled.0, Some("div".to_string()));
    assert_eq!(
        root_div.state.bubbled.1,
        vec![
            Box::new(BubbledUpStateTester(Some("p".to_string()), Vec::new())),
            Box::new(BubbledUpStateTester(Some("h1".to_string()), Vec::new()))
        ]
    );
    assert_eq!(root_div.state.pushed.0, Some("div".to_string()));
    assert_eq!(
        root_div.state.pushed.1,
        Some(Box::new(PushedDownStateTester(
            Some("Root".to_string()),
            None
        )))
    );
    assert_eq!(root_div.state.node.0, Some("div".to_string()));
    assert_eq!(root_div.state.node.1, vec![]);

    let child_p_id = dom.children_ids(root_div_id).unwrap()[0];
    let child_p = &dom[child_p_id];
    assert_eq!(child_p.state.bubbled.0, Some("p".to_string()));
    assert_eq!(child_p.state.bubbled.1, Vec::new());
    assert_eq!(child_p.state.pushed.0, Some("p".to_string()));
    assert_eq!(
        child_p.state.pushed.1,
        Some(Box::new(PushedDownStateTester(
            Some("div".to_string()),
            Some(Box::new(PushedDownStateTester(
                Some("Root".to_string()),
                None
            )))
        )))
    );
    assert_eq!(child_p.state.node.0, Some("p".to_string()));
    assert_eq!(
        child_p.state.node.1,
        vec![("color".to_string(), "red".to_string())]
    );

    let child_h1_id = dom.children_ids(root_div_id).unwrap()[1];
    let child_h1 = &dom[child_h1_id];
    assert_eq!(child_h1.state.bubbled.0, Some("h1".to_string()));
    assert_eq!(child_h1.state.bubbled.1, Vec::new());
    assert_eq!(child_h1.state.pushed.0, Some("h1".to_string()));
    assert_eq!(
        child_h1.state.pushed.1,
        Some(Box::new(PushedDownStateTester(
            Some("div".to_string()),
            Some(Box::new(PushedDownStateTester(
                Some("Root".to_string()),
                None
            )))
        )))
    );
    assert_eq!(child_h1.state.node.0, Some("h1".to_string()));
    assert_eq!(child_h1.state.node.1, vec![]);
}

#[test]
fn state_reduce_parent_called_minimally_on_update() {
    #[allow(non_snake_case)]
    fn Base(cx: Scope) -> Element {
        let width = if cx.generation() == 0 { "100%" } else { "99%" };
        cx.render(rsx! {
            div {
                width: "{width}",
                div{
                    div{
                        p{}
                    }
                    p{
                        "hello"
                    }
                    div{
                        h1{}
                    }
                    p{
                        "world"
                    }
                }
            }
        })
    }

    let mut vdom = VirtualDom::new(Base);

    let mut dom: RealDom<CallCounterState> = RealDom::new();

    let (nodes_updated, _) = dom.apply_mutations(vdom.rebuild());
    let _to_rerender = dom.update_state(nodes_updated, SendAnyMap::new());
    vdom.mark_dirty(ScopeId(0));
    let (nodes_updated, _) = dom.apply_mutations(vdom.render_immediate());
    let _to_rerender = dom.update_state(nodes_updated, SendAnyMap::new());

    let mut is_root = true;
    dom.traverse_depth_first(|n| {
        if is_root {
            is_root = false;
            assert_eq!(n.state.parent_counter.0, 1);
        } else {
            assert_eq!(n.state.parent_counter.0, 2);
        }
    });
}

#[test]
fn state_reduce_child_called_minimally_on_update() {
    #[allow(non_snake_case)]
    fn Base(cx: Scope) -> Element {
        let width = if cx.generation() == 0 { "100%" } else { "99%" };
        cx.render(rsx! {
            // updated: 2
            div {
                // updated: 2
                div{
                    // updated: 2
                    div{
                        // updated: 2
                        p{
                            width: "{width}",
                        }
                    }
                    // updated: 1
                    p{
                        // updated: 1
                        "hello"
                    }
                    // updated: 1
                    div{
                        // updated: 1
                        h1{}
                    }
                    // updated: 1
                    p{
                        // updated: 1
                        "world"
                    }
                }
            }
        })
    }

    let mut vdom = VirtualDom::new(Base);

    let mut dom: RealDom<CallCounterState> = RealDom::new();

    let (nodes_updated, _) = dom.apply_mutations(vdom.rebuild());
    let _to_rerender = dom.update_state(nodes_updated, SendAnyMap::new());
    vdom.mark_dirty(ScopeId(0));
    let (nodes_updated, _) = dom.apply_mutations(vdom.render_immediate());
    let _to_rerender = dom.update_state(nodes_updated, SendAnyMap::new());

    let mut traverse_count = 0;
    dom.traverse_depth_first(|n| {
        assert_eq!(n.state.child_counter.0, {
            if traverse_count > 4 {
                1
            } else {
                2
            }
        });
        traverse_count += 1;
    });
}

#[derive(Debug, Clone, Default, State)]
struct UnorderedDependanciesState {
    #[node_dep_state(c)]
    b: BDepCallCounter,
    #[node_dep_state()]
    c: CDepCallCounter,
    #[node_dep_state(b)]
    a: ADepCallCounter,
}

#[derive(Debug, Clone, Default, PartialEq)]
struct ADepCallCounter(usize, BDepCallCounter);
impl NodeDepState for ADepCallCounter {
    type Ctx = ();
    type DepState = (BDepCallCounter,);
    const NODE_MASK: NodeMask = NodeMask::NONE;
    fn reduce(
        &mut self,
        _node: NodeView,
        (sibling,): (&BDepCallCounter,),
        _ctx: &Self::Ctx,
    ) -> bool {
        self.0 += 1;
        self.1 = sibling.clone();
        true
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
struct BDepCallCounter(usize, CDepCallCounter);
impl NodeDepState for BDepCallCounter {
    type Ctx = ();
    type DepState = (CDepCallCounter,);
    const NODE_MASK: NodeMask = NodeMask::NONE;
    fn reduce(
        &mut self,
        _node: NodeView,
        (sibling,): (&CDepCallCounter,),
        _ctx: &Self::Ctx,
    ) -> bool {
        self.0 += 1;
        self.1 = sibling.clone();
        true
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
struct CDepCallCounter(usize);
impl NodeDepState for CDepCallCounter {
    type Ctx = ();
    type DepState = ();
    const NODE_MASK: NodeMask = NodeMask::ALL;
    fn reduce(&mut self, _node: NodeView, _sibling: (), _ctx: &Self::Ctx) -> bool {
        self.0 += 1;
        true
    }
}

#[test]
fn dependancies_order_independant() {
    #[allow(non_snake_case)]
    fn Base(cx: Scope) -> Element {
        render!(div {
            width: "100%",
            p{
                "hello"
            }
        })
    }

    let mut vdom = VirtualDom::new(Base);

    let mut dom: RealDom<UnorderedDependanciesState> = RealDom::new();

    let mutations = vdom.rebuild();
    let (nodes_updated, _) = dom.apply_mutations(mutations);
    let _to_rerender = dom.update_state(nodes_updated, SendAnyMap::new());

    let c = CDepCallCounter(1);
    let b = BDepCallCounter(1, c.clone());
    let a = ADepCallCounter(1, b.clone());
    dom.traverse_depth_first(|n| {
        assert_eq!(&n.state.a, &a);
        assert_eq!(&n.state.b, &b);
        assert_eq!(&n.state.c, &c);
    });
}
