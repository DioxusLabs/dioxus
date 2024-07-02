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

    let template = &o.template.get();

    assert_eq!(template.attr_paths.len(), 3);

    let _circle = template.roots[0];
    let TemplateNode::Element { attrs, tag, namespace, children } = _circle else {
        panic!("Expected an element");
    };

    assert_eq!(tag, "circle");
    assert_eq!(namespace, Some("http://www.w3.org/2000/svg"));
    assert_eq!(children.len(), 0);
    assert_eq!(attrs.len(), 5);
}
