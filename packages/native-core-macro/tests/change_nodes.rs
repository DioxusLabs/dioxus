use dioxus::core::{self as dioxus_core, GlobalNodeId};
use dioxus::prelude::*;
use dioxus_native_core::real_dom::RealDom;
use dioxus_native_core::state::State;
use dioxus_native_core_macro::State;

#[derive(State, Default, Clone)]
struct Empty {}

#[test]
fn remove_node() {
    #[allow(non_snake_case)]
    fn Base(cx: Scope) -> Element {
        render!(div {})
    }

    let vdom = VirtualDom::new(Base);

    let mut dom: RealDom<Empty> = RealDom::new();
    let (create, edit) = vdom.diff_lazynodes(
        rsx! {
            div{
                div{}
            }
        },
        rsx! {
            div{}
        },
    );

    println!("create: {:#?}", create);
    println!("edit: {:#?}", edit);

    let _to_update = dom.apply_mutations(vec![create]);

    assert_eq!(
        dom[GlobalNodeId::TemplateId {
            template_ref_id: dioxus_core::ElementId(1),
            template_node_id: dioxus::prelude::TemplateNodeId(0),
        }]
        .node_data
        .height,
        1
    );
    assert_eq!(
        dom[GlobalNodeId::TemplateId {
            template_ref_id: dioxus_core::ElementId(1),
            template_node_id: dioxus::prelude::TemplateNodeId(1),
        }]
        .node_data
        .height,
        2
    );

    dom.apply_mutations(vec![edit]);

    assert_eq!(dom.size(), 1);
    assert_eq!(
        dom[GlobalNodeId::TemplateId {
            template_ref_id: dioxus_core::ElementId(2),
            template_node_id: dioxus::prelude::TemplateNodeId(0),
        }]
        .node_data
        .height,
        1
    );
}

#[test]
fn add_node() {
    #[allow(non_snake_case)]
    fn Base(cx: Scope) -> Element {
        render!(div {})
    }

    let vdom = VirtualDom::new(Base);

    let (create, update) = vdom.diff_lazynodes(
        rsx! {
            div{}
        },
        rsx! {
            div{
                p{}
            }
        },
    );

    let mut dom: RealDom<Empty> = RealDom::new();

    let _to_update = dom.apply_mutations(vec![create]);

    assert_eq!(dom.size(), 1);
    assert_eq!(
        dom[GlobalNodeId::TemplateId {
            template_ref_id: dioxus_core::ElementId(1),
            template_node_id: dioxus::prelude::TemplateNodeId(0),
        }]
        .node_data
        .height,
        1
    );

    dom.apply_mutations(vec![update]);

    assert_eq!(dom.size(), 1);
    assert_eq!(
        dom[GlobalNodeId::TemplateId {
            template_ref_id: dioxus_core::ElementId(2),
            template_node_id: dioxus::prelude::TemplateNodeId(0),
        }]
        .node_data
        .height,
        1
    );
    assert_eq!(
        dom[GlobalNodeId::TemplateId {
            template_ref_id: dioxus_core::ElementId(2),
            template_node_id: dioxus::prelude::TemplateNodeId(1),
        }]
        .node_data
        .height,
        2
    );
}
