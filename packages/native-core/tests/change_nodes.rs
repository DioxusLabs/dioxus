use dioxus_core::VNode;
use dioxus_core::*;
use dioxus_core_macro::*;
use dioxus_html as dioxus_elements;
use dioxus_native_core::real_dom::RealDom;
use dioxus_native_core::real_dom_new_api::State;
use dioxus_native_core_macro::State;
use std::cell::Cell;

#[derive(State, Default, Clone)]
struct Empty {}

#[test]
fn remove_node() {
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

    let mut dom: RealDom<Empty> = RealDom::new();

    let _to_update = dom.apply_mutations(vec![mutations]);
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

    assert_eq!(dom.size(), 2);
    assert!(&dom.contains_node(&VNode::Element(&root_div)));
    assert_eq!(dom[1].height, 1);
    assert_eq!(dom[2].height, 2);

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
    dom.apply_mutations(vec![mutations.1]);

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

    assert_eq!(dom.size(), 1);
    assert!(&dom.contains_node(&VNode::Element(&new_root_div)));
    assert_eq!(dom[1].height, 1);
}

#[test]
fn add_node() {
    #[allow(non_snake_case)]
    fn Base(cx: Scope) -> Element {
        rsx!(cx, div {})
    }

    let vdom = VirtualDom::new(Base);

    let mutations = vdom.create_vnodes(rsx! {
        div{}
    });

    let mut dom: RealDom<Empty> = RealDom::new();

    let _to_update = dom.apply_mutations(vec![mutations]);

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

    assert_eq!(dom.size(), 1);
    assert!(&dom.contains_node(&VNode::Element(&root_div)));
    assert_eq!(dom[1].height, 1);

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
    dom.apply_mutations(vec![mutations.1]);

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

    assert_eq!(dom.size(), 2);
    assert!(&dom.contains_node(&VNode::Element(&new_root_div)));
    assert_eq!(dom[1].height, 1);
    assert_eq!(dom[2].height, 2);
}
