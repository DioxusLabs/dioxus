use dioxus::prelude::*;

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

    assert_eq!(template.attr_paths().count(), 3);

    let circle = template
        .root_op_index(0)
        .expect("expected one static root element");
    let (skip, namespace) = template.enter_meta(circle).expect("Expected an element op");

    assert_eq!(template.static_string_at_op(circle + 1), Some("circle"));
    assert!(namespace);
    assert_eq!(
        template.static_string_at_op(circle + 2),
        Some("http://www.w3.org/2000/svg")
    );

    let mut cursor = template
        .element_children_start(circle)
        .expect("element op should have metadata");
    let mut attr_count = 0;
    while cursor < circle + skip {
        if let Some(len) = template.attr_op_len(cursor) {
            attr_count += 1;
            cursor += len;
        } else if template.dynamic_op_is_attr(cursor) {
            cursor += 1;
        } else {
            break;
        }
    }
    assert_eq!(attr_count + template.attr_paths().count(), 5);
}
