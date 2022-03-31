use dioxus_core::VNode;
use dioxus_core::*;
use dioxus_core_macro::*;
use dioxus_html as dioxus_elements;
use dioxus_native_core::real_dom::*;

#[derive(Debug, Clone, PartialEq, Default)]
struct CallCounter(u32);
impl BubbledUpState for CallCounter {
    type Ctx = ();

    fn reduce<'a, I>(&mut self, _children: I, _vnode: &VNode, _ctx: &mut Self::Ctx)
    where
        I: Iterator<Item = &'a Self>,
        Self: 'a,
    {
        self.0 += 1;
    }
}

impl PushedDownState for CallCounter {
    type Ctx = ();

    fn reduce(&mut self, _parent: Option<&Self>, _vnode: &VNode, _ctx: &mut Self::Ctx) {
        self.0 += 1;
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
struct BubbledUpStateTester(String, Vec<Box<BubbledUpStateTester>>);
impl BubbledUpState for BubbledUpStateTester {
    type Ctx = u32;

    fn reduce<'a, I>(&mut self, children: I, vnode: &VNode, ctx: &mut Self::Ctx)
    where
        I: Iterator<Item = &'a Self>,
        Self: 'a,
    {
        assert_eq!(*ctx, 42);
        *self = BubbledUpStateTester(
            vnode.mounted_id().to_string(),
            children.map(|c| Box::new(c.clone())).collect(),
        );
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
struct PushedDownStateTester(String, Option<Box<PushedDownStateTester>>);
impl PushedDownState for PushedDownStateTester {
    type Ctx = u32;

    fn reduce(&mut self, parent: Option<&Self>, vnode: &VNode, ctx: &mut Self::Ctx) {
        assert_eq!(*ctx, 42);
        *self = PushedDownStateTester(
            vnode.mounted_id().to_string(),
            parent.map(|c| Box::new(c.clone())),
        );
    }
}

#[test]
fn tree_state_initial() {
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
            p{}
            h1{}
        }
    });

    let mut tree: RealDom<BubbledUpStateTester, PushedDownStateTester> = RealDom::new();

    let nodes_updated = tree.apply_mutations(vec![mutations]);
    let _to_rerender = tree.update_state(&vdom, nodes_updated, &mut 42, &mut 42);

    let root_div = &tree[1];
    assert_eq!(root_div.up_state.0, "1");
    assert_eq!(
        root_div.up_state.1,
        vec![
            Box::new(BubbledUpStateTester("2".to_string(), Vec::new())),
            Box::new(BubbledUpStateTester("3".to_string(), Vec::new()))
        ]
    );
    assert_eq!(root_div.down_state.0, "1");
    assert_eq!(root_div.down_state.1, None);

    let child_p = &tree[2];
    assert_eq!(child_p.up_state.0, "2");
    assert_eq!(child_p.up_state.1, Vec::new());
    assert_eq!(child_p.down_state.0, "2");
    assert_eq!(
        child_p.down_state.1,
        Some(Box::new(PushedDownStateTester("1".to_string(), None)))
    );

    let child_h1 = &tree[3];
    assert_eq!(child_h1.up_state.0, "3");
    assert_eq!(child_h1.up_state.1, Vec::new());
    assert_eq!(child_h1.down_state.0, "3");
    assert_eq!(
        child_h1.down_state.1,
        Some(Box::new(PushedDownStateTester("1".to_string(), None)))
    );
}

#[test]
fn tree_state_reduce_initally_called_minimally() {
    #[derive(Debug, Clone, PartialEq, Default)]
    struct CallCounter(u32);
    impl BubbledUpState for CallCounter {
        type Ctx = ();

        fn reduce<'a, I>(&mut self, _children: I, _vnode: &VNode, _ctx: &mut Self::Ctx)
        where
            I: Iterator<Item = &'a Self>,
            Self: 'a,
        {
            self.0 += 1;
        }
    }

    impl PushedDownState for CallCounter {
        type Ctx = ();

        fn reduce(&mut self, _parent: Option<&Self>, _vnode: &VNode, _ctx: &mut Self::Ctx) {
            self.0 += 1;
        }
    }

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

    let mut tree: RealDom<CallCounter, CallCounter> = RealDom::new();

    let nodes_updated = tree.apply_mutations(vec![mutations]);
    let _to_rerender = tree.update_state(&vdom, nodes_updated, &mut (), &mut ());

    tree.traverse_depth_first(|n| {
        assert_eq!(n.up_state.0, 1);
        assert_eq!(n.down_state.0, 1);
    });
}

#[test]
fn tree_state_reduce_down_called_minimally_on_update() {
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

    let mut tree: RealDom<CallCounter, CallCounter> = RealDom::new();

    let nodes_updated = tree.apply_mutations(vec![mutations]);
    let _to_rerender = tree.update_state(&vdom, nodes_updated, &mut (), &mut ());
    let nodes_updated = tree.apply_mutations(vec![Mutations {
        edits: vec![DomEdit::SetAttribute {
            root: 1,
            field: "width",
            value: "99%",
            ns: Some("style"),
        }],
        dirty_scopes: fxhash::FxHashSet::default(),
        refs: Vec::new(),
    }]);
    let _to_rerender = tree.update_state(&vdom, nodes_updated, &mut (), &mut ());

    tree.traverse_depth_first(|n| {
        assert_eq!(n.down_state.0, 2);
    });
}

#[test]
fn tree_state_reduce_up_called_minimally_on_update() {
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

    let mut tree: RealDom<CallCounter, CallCounter> = RealDom::new();

    let nodes_updated = tree.apply_mutations(vec![mutations]);
    let _to_rerender = tree.update_state(&vdom, nodes_updated, &mut (), &mut ());
    let nodes_updated = tree.apply_mutations(vec![Mutations {
        edits: vec![DomEdit::SetAttribute {
            root: 4,
            field: "width",
            value: "99%",
            ns: Some("style"),
        }],
        dirty_scopes: fxhash::FxHashSet::default(),
        refs: Vec::new(),
    }]);
    let _to_rerender = tree.update_state(&vdom, nodes_updated, &mut (), &mut ());

    tree.traverse_depth_first(|n| {
        assert_eq!(n.up_state.0, if n.id.0 > 4 { 1 } else { 2 });
    });
}
