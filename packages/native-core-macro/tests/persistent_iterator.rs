use dioxus::prelude::*;
use dioxus_native_core::{
    real_dom::{ RealDom},
    node::NodeType,
    state::State,
    utils::PersistantElementIter,
};
use dioxus_native_core_macro::State;

#[derive(State, Default, Clone, Debug)]
struct Empty {}

#[test]
#[allow(unused_variables)]
fn traverse() {
    #[allow(non_snake_case)]
    fn Base(cx: Scope) -> Element {
        render!(
            div{
                div{
                    "hello"
                    p{
                        "world"
                    }
                    "hello world"
                }
            }
        )
    }

    let mut vdom = VirtualDom::new(Base);
    let mutations = vdom.rebuild();

    let mut rdom: RealDom<Empty> = RealDom::new();

    let _to_update = rdom.apply_mutations(mutations);

    let mut iter = PersistantElementIter::new();
    let div_tag = "div".to_string();
    assert!(matches!(
        &rdom[iter.next(&rdom).id()].node_data.node_type,
        NodeType::Element { tag: div_tag, .. }
    ));
    assert!(matches!(
        &rdom[iter.next(&rdom).id()].node_data.node_type,
        NodeType::Element { tag: div_tag, .. }
    ));
    let text1 = "hello".to_string();
    assert!(matches!(
        &rdom[iter.next(&rdom).id()].node_data.node_type,
        NodeType::Text { text: text1, .. }
    ));
    let p_tag = "p".to_string();
    assert!(matches!(
        &rdom[iter.next(&rdom).id()].node_data.node_type,
        NodeType::Element { tag: p_tag, .. }
    ));
    let text2 = "world".to_string();
    assert!(matches!(
        &rdom[iter.next(&rdom).id()].node_data.node_type,
        NodeType::Text { text: text2, .. }
    ));
    let text3 = "hello world".to_string();
    assert!(matches!(
        &rdom[iter.next(&rdom).id()].node_data.node_type,
        NodeType::Text { text: text3, .. }
    ));
    assert!(matches!(
        &rdom[iter.next(&rdom).id()].node_data.node_type,
        NodeType::Element { tag: div_tag, .. }
    ));

    assert!(matches!(
        &rdom[iter.prev(&rdom).id()].node_data.node_type,
        NodeType::Text { text: text3, .. }
    ));
    assert!(matches!(
        &rdom[iter.prev(&rdom).id()].node_data.node_type,
        NodeType::Text { text: text2, .. }
    ));
    assert!(matches!(
        &rdom[iter.prev(&rdom).id()].node_data.node_type,
        NodeType::Element { tag: p_tag, .. }
    ));
    assert!(matches!(
        &rdom[iter.prev(&rdom).id()].node_data.node_type,
        NodeType::Text { text: text1, .. }
    ));
    assert!(matches!(
        &rdom[iter.prev(&rdom).id()].node_data.node_type,
        NodeType::Element { tag: div_tag, .. }
    ));
    assert!(matches!(
        &rdom[iter.prev(&rdom).id()].node_data.node_type,
        NodeType::Element { tag: div_tag, .. }
    ));
    assert!(matches!(
        &rdom[iter.prev(&rdom).id()].node_data.node_type,
        NodeType::Element { tag: div_tag, .. }
    ));
    assert!(matches!(
        &rdom[iter.prev(&rdom).id()].node_data.node_type,
        NodeType::Text { text: text3, .. }
    ));
}

#[test]
#[allow(unused_variables)]
fn persist_removes() {
    #[allow(non_snake_case)]
    fn Base(cx: Scope) -> Element {
        let children = match cx.generation()%2{
            0=>3,
            1=>2,
            _ => unreachable!()
        };
        render!(
            div{
                (0..children).map(|i|{
                    rsx!{
                        p{
                            key: "{i}",
                            "{i}"
                        }
                    }
                })
            }
        )
    }
    let mut vdom = VirtualDom::new(Base);

    let mut rdom: RealDom<Empty> = RealDom::new();

    let build = vdom.rebuild();
    let _to_update = rdom.apply_mutations(build);

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
    // "1"
    iter1.next(&rdom).id();
    iter2.next(&rdom).id();
    // p
    iter1.next(&rdom).id();
    iter2.next(&rdom).id();
    // "2"
    iter1.next(&rdom).id();
    iter2.next(&rdom).id();
    // p
    iter2.next(&rdom).id();
    // "3"
    iter2.next(&rdom).id();

    let update = vdom.rebuild();
    iter1.prune(&update, &rdom);
    iter2.prune(&update, &rdom);
    let _to_update = rdom.apply_mutations(update);

    let p_tag = "1".to_string();
    let idx = iter1.next(&rdom).id();
    assert!(matches!(
        &rdom[idx].node_data.node_type,
        NodeType::Element { tag: p_tag, .. }
    ));
    let text = "2".to_string();
    let idx = iter1.next(&rdom).id();
    assert!(matches!(
        &rdom[idx].node_data.node_type,
        NodeType::Text { text, .. }
    ));
    let div_tag = "div".to_string();
    let idx = iter2.next(&rdom).id();
    assert!(matches!(
        &rdom[idx].node_data.node_type,
        NodeType::Element { tag: div_tag, .. }
    ));
}

#[test]
#[allow(unused_variables)]
fn persist_instertions_before() {
    #[allow(non_snake_case)]
    fn Base(cx: Scope) -> Element {
        let children = match cx.generation()%2{
            0=>3,
            1=>2,
            _ => unreachable!()
        };
        render!(
            div{
                (0..children).map(|i|{
                    rsx!{
                        p{
                            key: "{i}",
                            "{i}"
                        }
                    }
                })
            }
        )
    }
    let mut vdom = VirtualDom::new(Base);

    let mut rdom: RealDom<Empty> = RealDom::new();

    let build = vdom.rebuild();
    let _to_update = rdom.apply_mutations(build);
    
    let mut iter = PersistantElementIter::new();
    // div
    iter.next(&rdom).id();
    // p
    iter.next(&rdom).id();
    // "1"
    iter.next(&rdom).id();
    // p
    iter.next(&rdom).id();
    // "2"
    iter.next(&rdom).id();
    
    let update = vdom.rebuild();
    iter.prune(&update, &rdom);
    let _to_update = rdom.apply_mutations(update);

    let p_tag = "div".to_string();
    let idx = iter.next(&rdom).id();
    assert!(matches!(
        &rdom[idx].node_data.node_type,
        NodeType::Element { tag: p_tag, .. }
    ));
}

#[test]
#[allow(unused_variables)]
fn persist_instertions_after() {
    #[allow(non_snake_case)]
    fn Base(cx: Scope) -> Element {
        let children = match cx.generation()%2{
            0=>3,
            1=>2,
            _ => unreachable!()
        };
        render!(
            div{
                (0..children).map(|i|{
                    rsx!{
                        p{
                            key: "{i}",
                            "{i}"
                        }
                    }
                })
            }
        )
    }
    let mut vdom = VirtualDom::new(Base);

    let mut rdom: RealDom<Empty> = RealDom::new();

    let build = vdom.rebuild();
    let _to_update = rdom.apply_mutations(build);

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

    let update = vdom.rebuild();
    iter.prune(&update, &rdom);
    let _to_update = rdom.apply_mutations(update);

    let p_tag = "p".to_string();
    let idx = iter.next(&rdom).id();
    assert!(matches!(
        &rdom[idx].node_data.node_type,
        NodeType::Element { tag: p_tag, .. }
    ));
    let text = "hello world".to_string();
    let idx = iter.next(&rdom).id();
    assert!(matches!(
        &rdom[idx].node_data.node_type,
        NodeType::Text { text, .. }
    ));
}
