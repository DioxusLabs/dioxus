use dioxus::core as dioxus_core;
use dioxus::prelude::*;
use dioxus_native_core::{
    real_dom::{NodeType, RealDom},
    state::State,
    utils::PersistantElementIter,
};
use dioxus_native_core_macro::State;

#[derive(State, Default, Clone)]
struct Empty {}

#[test]
#[allow(unused_variables)]
fn traverse() {
    #[allow(non_snake_case)]
    fn Base(cx: Scope) -> Element {
        render!(div {})
    }
    let vdom = VirtualDom::new(Base);
    let mutations = vdom.create_vnodes(rsx! {
        div{
            div{
                "hello"
                p{
                    "world"
                }
                "hello world"
            }
        }
    });

    let mut rdom: RealDom<Empty> = RealDom::new();

    let _to_update = rdom.apply_mutations(vec![mutations]);

    let mut iter = PersistantElementIter::new();
    let div_tag = "div".to_string();
    assert!(matches!(
        &rdom[iter.next(&rdom).id()].node_type,
        NodeType::Element { tag: div_tag, .. }
    ));
    assert!(matches!(
        &rdom[iter.next(&rdom).id()].node_type,
        NodeType::Element { tag: div_tag, .. }
    ));
    let text1 = "hello".to_string();
    assert!(matches!(
        &rdom[iter.next(&rdom).id()].node_type,
        NodeType::Text { text: text1, .. }
    ));
    let p_tag = "p".to_string();
    assert!(matches!(
        &rdom[iter.next(&rdom).id()].node_type,
        NodeType::Element { tag: p_tag, .. }
    ));
    let text2 = "world".to_string();
    assert!(matches!(
        &rdom[iter.next(&rdom).id()].node_type,
        NodeType::Text { text: text2, .. }
    ));
    let text3 = "hello world".to_string();
    assert!(matches!(
        &rdom[iter.next(&rdom).id()].node_type,
        NodeType::Text { text: text3, .. }
    ));
    assert!(matches!(
        &rdom[iter.next(&rdom).id()].node_type,
        NodeType::Element { tag: div_tag, .. }
    ));

    assert!(matches!(
        &rdom[iter.prev(&rdom).id()].node_type,
        NodeType::Text { text: text3, .. }
    ));
    assert!(matches!(
        &rdom[iter.prev(&rdom).id()].node_type,
        NodeType::Text { text: text2, .. }
    ));
    assert!(matches!(
        &rdom[iter.prev(&rdom).id()].node_type,
        NodeType::Element { tag: p_tag, .. }
    ));
    assert!(matches!(
        &rdom[iter.prev(&rdom).id()].node_type,
        NodeType::Text { text: text1, .. }
    ));
    assert!(matches!(
        &rdom[iter.prev(&rdom).id()].node_type,
        NodeType::Element { tag: div_tag, .. }
    ));
    assert!(matches!(
        &rdom[iter.prev(&rdom).id()].node_type,
        NodeType::Element { tag: div_tag, .. }
    ));
    assert!(matches!(
        &rdom[iter.prev(&rdom).id()].node_type,
        NodeType::Element { tag: div_tag, .. }
    ));
    assert!(matches!(
        &rdom[iter.prev(&rdom).id()].node_type,
        NodeType::Text { text: text3, .. }
    ));
}

#[test]
#[allow(unused_variables)]
fn persist_removes() {
    #[allow(non_snake_case)]
    fn Base(cx: Scope) -> Element {
        render!(div {})
    }
    let vdom = VirtualDom::new(Base);
    let (build, update) = vdom.diff_lazynodes(
        rsx! {
            div{
                p{
                    key: "1",
                    "hello"
                }
                p{
                    key: "2",
                    "world"
                }
                p{
                    key: "3",
                    "hello world"
                }
            }
        },
        rsx! {
            div{
                p{
                    key: "1",
                    "hello"
                }
                p{
                    key: "3",
                    "hello world"
                }
            }
        },
    );

    let mut rdom: RealDom<Empty> = RealDom::new();

    let _to_update = rdom.apply_mutations(vec![build]);

    // this will end on the node that is removed
    let mut iter1 = PersistantElementIter::new();
    // this will end on the after node that is removed
    let mut iter2 = PersistantElementIter::new();
    // div
    iter1.next(&rdom).id();
    iter2.next(&rdom).id();
    // p
    iter1.next(&rdom).id();
    iter2.next(&rdom).id();
    // "hello"
    iter1.next(&rdom).id();
    iter2.next(&rdom).id();
    // p
    iter1.next(&rdom).id();
    iter2.next(&rdom).id();
    // "world"
    iter1.next(&rdom).id();
    iter2.next(&rdom).id();
    // p
    iter2.next(&rdom).id();
    // "hello world"
    iter2.next(&rdom).id();

    iter1.prune(&update, &rdom);
    iter2.prune(&update, &rdom);
    let _to_update = rdom.apply_mutations(vec![update]);

    let p_tag = "p".to_string();
    let idx = iter1.next(&rdom).id();
    assert!(matches!(
        &rdom[idx].node_type,
        NodeType::Element { tag: p_tag, .. }
    ));
    let text = "hello world".to_string();
    let idx = iter1.next(&rdom).id();
    assert!(matches!(&rdom[idx].node_type, NodeType::Text { text, .. }));
    let div_tag = "div".to_string();
    let idx = iter2.next(&rdom).id();
    assert!(matches!(
        &rdom[idx].node_type,
        NodeType::Element { tag: div_tag, .. }
    ));
}

#[test]
#[allow(unused_variables)]
fn persist_instertions_before() {
    #[allow(non_snake_case)]
    fn Base(cx: Scope) -> Element {
        render!(div {})
    }
    let vdom = VirtualDom::new(Base);
    let (build, update) = vdom.diff_lazynodes(
        rsx! {
            div{
                p{
                    key: "1",
                    "hello"
                }
                p{
                    key: "3",
                    "hello world"
                }
            }
        },
        rsx! {
            div{
                p{
                    key: "1",
                    "hello"
                }
                p{
                    key: "2",
                    "world"
                }
                p{
                    key: "3",
                    "hello world"
                }
            }
        },
    );

    let mut rdom: RealDom<Empty> = RealDom::new();

    let _to_update = rdom.apply_mutations(vec![build]);

    let mut iter = PersistantElementIter::new();
    // div
    iter.next(&rdom).id();
    // p
    iter.next(&rdom).id();
    // "hello"
    iter.next(&rdom).id();
    // p
    iter.next(&rdom).id();
    // "hello world"
    iter.next(&rdom).id();

    iter.prune(&update, &rdom);
    let _to_update = rdom.apply_mutations(vec![update]);

    let p_tag = "div".to_string();
    let idx = iter.next(&rdom).id();
    assert!(matches!(
        &rdom[idx].node_type,
        NodeType::Element { tag: p_tag, .. }
    ));
}

#[test]
#[allow(unused_variables)]
fn persist_instertions_after() {
    #[allow(non_snake_case)]
    fn Base(cx: Scope) -> Element {
        render!(div {})
    }
    let vdom = VirtualDom::new(Base);
    let (build, update) = vdom.diff_lazynodes(
        rsx! {
            div{
                p{
                    key: "1",
                    "hello"
                }
                p{
                    key: "2",
                    "world"
                }
            }
        },
        rsx! {
            div{
                p{
                    key: "1",
                    "hello"
                }
                p{
                    key: "2",
                    "world"
                }
                p{
                    key: "3",
                    "hello world"
                }
            }
        },
    );

    let mut rdom: RealDom<Empty> = RealDom::new();

    let _to_update = rdom.apply_mutations(vec![build]);

    let mut iter = PersistantElementIter::new();
    // div
    iter.next(&rdom).id();
    // p
    iter.next(&rdom).id();
    // "hello"
    iter.next(&rdom).id();
    // p
    iter.next(&rdom).id();
    // "world"
    iter.next(&rdom).id();

    iter.prune(&update, &rdom);
    let _to_update = rdom.apply_mutations(vec![update]);

    let p_tag = "p".to_string();
    let idx = iter.next(&rdom).id();
    assert!(matches!(
        &rdom[idx].node_type,
        NodeType::Element { tag: p_tag, .. }
    ));
    let text = "hello world".to_string();
    let idx = iter.next(&rdom).id();
    assert!(matches!(&rdom[idx].node_type, NodeType::Text { text, .. }));
}
