use dioxus_core::VNode;
use dioxus_core::*;
use dioxus_core_macro::*;
use dioxus_html as dioxus_elements;
use dioxus_native_core::client_tree::ClientTree;
use std::cell::Cell;

#[test]
fn tree_remove_node() {
    #[allow(non_snake_case)]
    fn Base(cx: Scope) -> Element {
        rsx!(cx, div {})
    }

    let vdom = VirtualDom::new(Base);

    let mutations = vdom.create_vnodes(rsx! {
        div{
            div{}
        }
    });

    let mut tree: ClientTree<(), ()> = ClientTree::new();

    let _to_update = tree.apply_mutations(vec![mutations]);
    let child_div = VElement {
        id: Cell::new(Some(ElementId(2))),
        key: None,
        tag: "div",
        namespace: None,
        parent: Cell::new(Some(ElementId(1))),
        listeners: &[],
        attributes: &[],
        children: &[],
    };
    let child_div_el = VNode::Element(&child_div);
    let root_div = VElement {
        id: Cell::new(Some(ElementId(1))),
        key: None,
        tag: "div",
        namespace: None,
        parent: Cell::new(Some(ElementId(0))),
        listeners: &[],
        attributes: &[],
        children: &[child_div_el],
    };

    assert_eq!(tree.size(), 2);
    assert!(&tree.contains_node(&VNode::Element(&root_div)));
    assert_eq!(tree[1].height, 1);
    assert_eq!(tree[2].height, 2);

    let vdom = VirtualDom::new(Base);
    let mutations = vdom.diff_lazynodes(
        rsx! {
            div{
                div{}
            }
        },
        rsx! {
            div{}
        },
    );
    tree.apply_mutations(vec![mutations.1]);

    let new_root_div = VElement {
        id: Cell::new(Some(ElementId(1))),
        key: None,
        tag: "div",
        namespace: None,
        parent: Cell::new(Some(ElementId(0))),
        listeners: &[],
        attributes: &[],
        children: &[],
    };

    assert_eq!(tree.size(), 1);
    assert!(&tree.contains_node(&VNode::Element(&new_root_div)));
    assert_eq!(tree[1].height, 1);
}

#[test]
fn tree_add_node() {
    #[allow(non_snake_case)]
    fn Base(cx: Scope) -> Element {
        rsx!(cx, div {})
    }

    let vdom = VirtualDom::new(Base);

    let mutations = vdom.create_vnodes(rsx! {
        div{}
    });

    let mut tree: ClientTree<(), ()> = ClientTree::new();

    let _to_update = tree.apply_mutations(vec![mutations]);

    let root_div = VElement {
        id: Cell::new(Some(ElementId(1))),
        key: None,
        tag: "div",
        namespace: None,
        parent: Cell::new(Some(ElementId(0))),
        listeners: &[],
        attributes: &[],
        children: &[],
    };

    assert_eq!(tree.size(), 1);
    assert!(&tree.contains_node(&VNode::Element(&root_div)));
    assert_eq!(tree[1].height, 1);

    let vdom = VirtualDom::new(Base);
    let mutations = vdom.diff_lazynodes(
        rsx! {
            div{}
        },
        rsx! {
            div{
                p{}
            }
        },
    );
    tree.apply_mutations(vec![mutations.1]);

    let child_div = VElement {
        id: Cell::new(Some(ElementId(2))),
        key: None,
        tag: "p",
        namespace: None,
        parent: Cell::new(Some(ElementId(1))),
        listeners: &[],
        attributes: &[],
        children: &[],
    };
    let child_div_el = VNode::Element(&child_div);
    let new_root_div = VElement {
        id: Cell::new(Some(ElementId(1))),
        key: None,
        tag: "div",
        namespace: None,
        parent: Cell::new(Some(ElementId(0))),
        listeners: &[],
        attributes: &[],
        children: &[child_div_el],
    };

    assert_eq!(tree.size(), 2);
    assert!(&tree.contains_node(&VNode::Element(&new_root_div)));
    assert_eq!(tree[1].height, 1);
    assert_eq!(tree[2].height, 2);
}
