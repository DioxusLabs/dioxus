use dioxus::{dioxus_core::internal::TemplateExt, prelude::*};

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
    assert_eq!(template.dynamic_value_count(), 3);

    let circle = template
        .root_slots()
        .find_map(|(_, op, _)| op)
        .expect("expected one static root element");
    let (tag, namespace) = template
        .element_meta_at_op(circle)
        .expect("expected an element op");
    assert_eq!(tag, "circle");
    assert_eq!(namespace, Some("http://www.w3.org/2000/svg"));

    // Five attributes total: cx, cy, r (dynamic) and stroke, fill (static).
    let static_attr_count = template.static_attrs(circle).count();
    let dynamic_attr_count = template
        .element_dynamic_anchors(circle)
        .map(|anchor| o.dynamic_attr_indices_for_anchor(anchor).count())
        .sum::<usize>();
    assert_eq!(static_attr_count + dynamic_attr_count, 5);
}
