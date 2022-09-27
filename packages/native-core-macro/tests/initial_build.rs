use dioxus::core::{self as dioxus_core, GlobalNodeId};
use dioxus::prelude::*;
use dioxus_native_core::real_dom::RealDom;
use dioxus_native_core::state::State;
use dioxus_native_core_macro::State;

#[derive(Default, Clone, State)]
struct Empty {}

#[test]
fn initial_build_simple() {
    #[allow(non_snake_case)]
    fn Base(cx: Scope) -> Element {
        render!(div {})
    }

    let vdom = VirtualDom::new(Base);

    let mutations = vdom.create_vnodes(rsx! {
        div{}
    });

    let mut dom: RealDom<Empty> = RealDom::new();

    let _to_update = dom.apply_mutations(vec![mutations]);

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
}

#[test]
fn initial_build_with_children() {
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

    let mut dom: RealDom<Empty> = RealDom::new();

    let _to_update = dom.apply_mutations(vec![mutations]);
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
    assert_eq!(
        dom[GlobalNodeId::TemplateId {
            template_ref_id: dioxus_core::ElementId(1),
            template_node_id: dioxus::prelude::TemplateNodeId(1),
        }]
        .node_data
        .height,
        2
    );
    assert_eq!(
        dom[GlobalNodeId::TemplateId {
            template_ref_id: dioxus_core::ElementId(1),
            template_node_id: dioxus::prelude::TemplateNodeId(2),
        }]
        .node_data
        .height,
        3
    );
    assert_eq!(
        dom[GlobalNodeId::TemplateId {
            template_ref_id: dioxus_core::ElementId(1),
            template_node_id: dioxus::prelude::TemplateNodeId(3),
        }]
        .node_data
        .height,
        3
    );
    assert_eq!(
        dom[GlobalNodeId::TemplateId {
            template_ref_id: dioxus_core::ElementId(1),
            template_node_id: dioxus::prelude::TemplateNodeId(4),
        }]
        .node_data
        .height,
        4
    );
    assert_eq!(
        dom[GlobalNodeId::TemplateId {
            template_ref_id: dioxus_core::ElementId(1),
            template_node_id: dioxus::prelude::TemplateNodeId(5),
        }]
        .node_data
        .height,
        3
    );
}
