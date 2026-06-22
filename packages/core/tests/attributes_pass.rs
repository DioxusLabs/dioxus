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

    let template = &o.template;

    // The three numeric attributes (cx, cy, r) are dynamic; there are no dynamic nodes.
    assert_eq!(o.dynamic_values().len(), 3);

    let circle = o
        .children()
        .find_map(|child| match child {
            VNodeChild::Element(element) => Some(element.op()),
            _ => None,
        })
        .expect("expected one static root element");
    let (tag, namespace) = template
        .element_meta_at_op(circle)
        .expect("expected an element op");
    assert_eq!(tag, "circle");
    assert_eq!(namespace, Some("http://www.w3.org/2000/svg"));

    // Five attributes total: cx, cy, r (dynamic) and stroke, fill (static).
    let static_attr_count = template.static_attrs(circle).count();
    let dynamic_attr_count = o
        .dynamic_attributes()
        .filter(|group| group.parent_element_op_index() == circle)
        .map(|group| group.ids().count())
        .sum::<usize>();
    assert_eq!(static_attr_count + dynamic_attr_count, 5);
}
