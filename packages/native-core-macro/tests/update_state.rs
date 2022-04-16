use anymap::AnyMap;
use dioxus_core::VNode;
use dioxus_core::*;
use dioxus_core_macro::*;
use dioxus_html as dioxus_elements;
use dioxus_native_core::node_ref::*;
use dioxus_native_core::real_dom::*;
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
    fn reduce(&mut self, _node: NodeView, _sibling: &Self::DepState, _ctx: &Self::Ctx) -> bool {
        self.0 += 1;
        true
    }
}

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

#[test]
fn state_initial() {
    #[allow(non_snake_case)]
    fn Base(cx: Scope) -> Element {
        rsx!(cx, div {
            p{}
            h1{}
        })
    }

    let vdom = VirtualDom::new(Base);

    let mutations = vdom.create_vnodes(rsx! {
        div {
            p{
                color: "red"
            }
            h1{}
        }
    });

    let mut dom: RealDom<StateTester> = RealDom::new();

    let nodes_updated = dom.apply_mutations(vec![mutations]);
    let mut ctx = AnyMap::new();
    ctx.insert(42u32);
    let _to_rerender = dom.update_state(&vdom, nodes_updated, ctx);

    let root_div = &dom[1];
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
        Some(Box::new(PushedDownStateTester(None, None)))
    );
    assert_eq!(root_div.state.node.0, Some("div".to_string()));
    assert_eq!(root_div.state.node.1, vec![]);

    let child_p = &dom[2];
    assert_eq!(child_p.state.bubbled.0, Some("p".to_string()));
    assert_eq!(child_p.state.bubbled.1, Vec::new());
    assert_eq!(child_p.state.pushed.0, Some("p".to_string()));
    assert_eq!(
        child_p.state.pushed.1,
        Some(Box::new(PushedDownStateTester(
            Some("div".to_string()),
            Some(Box::new(PushedDownStateTester(None, None)))
        )))
    );
    assert_eq!(child_p.state.node.0, Some("p".to_string()));
    assert_eq!(
        child_p.state.node.1,
        vec![("color".to_string(), "red".to_string())]
    );

    let child_h1 = &dom[3];
    assert_eq!(child_h1.state.bubbled.0, Some("h1".to_string()));
    assert_eq!(child_h1.state.bubbled.1, Vec::new());
    assert_eq!(child_h1.state.pushed.0, Some("h1".to_string()));
    assert_eq!(
        child_h1.state.pushed.1,
        Some(Box::new(PushedDownStateTester(
            Some("div".to_string()),
            Some(Box::new(PushedDownStateTester(None, None)))
        )))
    );
    assert_eq!(child_h1.state.node.0, Some("h1".to_string()));
    assert_eq!(child_h1.state.node.1, vec![]);
}

#[test]
fn state_reduce_initally_called_minimally() {
    #[allow(non_snake_case)]
    fn Base(cx: Scope) -> Element {
        rsx!(cx, div {
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
        })
    }

    let vdom = VirtualDom::new(Base);

    let mutations = vdom.create_vnodes(rsx! {
        div {
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
    });

    let mut dom: RealDom<CallCounterState> = RealDom::new();

    let nodes_updated = dom.apply_mutations(vec![mutations]);
    let _to_rerender = dom.update_state(&vdom, nodes_updated, AnyMap::new());

    dom.traverse_depth_first(|n| {
        assert_eq!(n.state.child_counter.0, 1);
        assert_eq!(n.state.parent_counter.0, 1);
        assert_eq!(n.state.node_counter.0, 1);
    });
}

#[test]
fn state_reduce_parent_called_minimally_on_update() {
    #[allow(non_snake_case)]
    fn Base(cx: Scope) -> Element {
        rsx!(cx, div {
            width: "100%",
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
        })
    }

    let vdom = VirtualDom::new(Base);

    let mutations = vdom.create_vnodes(rsx! {
        div {
            width: "100%",
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
    });

    let mut dom: RealDom<CallCounterState> = RealDom::new();

    let nodes_updated = dom.apply_mutations(vec![mutations]);
    let _to_rerender = dom.update_state(&vdom, nodes_updated, AnyMap::new());
    let nodes_updated = dom.apply_mutations(vec![Mutations {
        edits: vec![DomEdit::SetAttribute {
            root: 1,
            field: "width",
            value: "99%",
            ns: Some("style"),
        }],
        dirty_scopes: fxhash::FxHashSet::default(),
        refs: Vec::new(),
    }]);
    let _to_rerender = dom.update_state(&vdom, nodes_updated, AnyMap::new());

    dom.traverse_depth_first(|n| {
        assert_eq!(n.state.parent_counter.0, 2);
    });
}

#[test]
fn state_reduce_child_called_minimally_on_update() {
    #[allow(non_snake_case)]
    fn Base(cx: Scope) -> Element {
        rsx!(cx, div {
            div{
                div{
                    p{
                        width: "100%",
                    }
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
        })
    }

    let vdom = VirtualDom::new(Base);

    let mutations = vdom.create_vnodes(rsx! {
        div {
            div{
                div{
                    p{
                        width: "100%",
                    }
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
    });

    let mut dom: RealDom<CallCounterState> = RealDom::new();

    let nodes_updated = dom.apply_mutations(vec![mutations]);
    let _to_rerender = dom.update_state(&vdom, nodes_updated, AnyMap::new());
    let nodes_updated = dom.apply_mutations(vec![Mutations {
        edits: vec![DomEdit::SetAttribute {
            root: 4,
            field: "width",
            value: "99%",
            ns: Some("style"),
        }],
        dirty_scopes: fxhash::FxHashSet::default(),
        refs: Vec::new(),
    }]);
    let _to_rerender = dom.update_state(&vdom, nodes_updated, AnyMap::new());

    dom.traverse_depth_first(|n| {
        println!("{:?}", n);
        assert_eq!(n.state.child_counter.0, if n.id.0 > 4 { 1 } else { 2 });
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
    type DepState = BDepCallCounter;
    const NODE_MASK: NodeMask = NodeMask::NONE;
    fn reduce(&mut self, _node: NodeView, sibling: &Self::DepState, _ctx: &Self::Ctx) -> bool {
        self.0 += 1;
        self.1 = sibling.clone();
        true
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
struct BDepCallCounter(usize, CDepCallCounter);
impl NodeDepState for BDepCallCounter {
    type Ctx = ();
    type DepState = CDepCallCounter;
    const NODE_MASK: NodeMask = NodeMask::NONE;
    fn reduce(&mut self, _node: NodeView, sibling: &Self::DepState, _ctx: &Self::Ctx) -> bool {
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
    fn reduce(&mut self, _node: NodeView, _sibling: &Self::DepState, _ctx: &Self::Ctx) -> bool {
        self.0 += 1;
        true
    }
}

#[test]
fn dependancies_order_independant() {
    #[allow(non_snake_case)]
    fn Base(cx: Scope) -> Element {
        rsx!(cx, div {
            width: "100%",
            p{
                "hello"
            }
        })
    }

    let vdom = VirtualDom::new(Base);

    let mutations = vdom.create_vnodes(rsx! {
        div {
            width: "100%",
            p{
                "hello"
            }
        }
    });

    let mut dom: RealDom<UnorderedDependanciesState> = RealDom::new();

    let nodes_updated = dom.apply_mutations(vec![mutations]);
    let _to_rerender = dom.update_state(&vdom, nodes_updated, AnyMap::new());

    let c = CDepCallCounter(1);
    let b = BDepCallCounter(1, c.clone());
    let a = ADepCallCounter(1, b.clone());
    dom.traverse_depth_first(|n| {
        assert_eq!(&n.state.a, &a);
        assert_eq!(&n.state.b, &b);
        assert_eq!(&n.state.c, &c);
    });
}
