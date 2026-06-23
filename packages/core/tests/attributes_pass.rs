use dioxus::prelude::*;
use dioxus_core::VNodeChild;

/// Make sure that rsx! is parsing templates and their attributes properly
#[test]
fn attributes_pass_properly() {
    let h = rsx! {
        circle {
            cx: 50,
            cy: 50,
            r: 40,
            stroke: "green",
            fill: "yellow"
        }
    };

    let o = h.unwrap();

    // The three numeric attributes (cx, cy, r) are dynamic; there are no dynamic nodes.
    assert_eq!(o.dynamic_node_values().len(), 0);
    assert_eq!(o.dynamic_attr_values().len(), 3);

    let circle = o
        .children()
        .find_map(|child| match child {
            VNodeChild::Element(element) => Some(element),
            _ => None,
        })
        .expect("expected one static root element");
    assert_eq!(circle.tag(), "circle");
    assert_eq!(circle.namespace(), Some("http://www.w3.org/2000/svg"));

    // Five attributes total: cx, cy, r (dynamic) and stroke, fill (static).
    let static_attr_count = circle.static_attributes().count();
    let dynamic_attr_count = o
        .dynamic_anchors()
        .filter(|anchor| anchor.parent_element_op_index() == Some(circle.op()))
        .flat_map(|anchor| anchor.attrs())
        .map(|slot| slot.attrs().len())
        .sum::<usize>();
    assert_eq!(static_attr_count + dynamic_attr_count, 5);
}
