use dioxus_rsx::{CallBody, DynamicContext};
use syn::Item;

#[test]
fn rsx_writeout_snapshot() {
    let body = parse_from_str(include_str!("./parsing/multiexpr.rsx"));

    assert_eq!(body.roots.len(), 1);

    let root = &body.roots[0];

    let el = match root {
        dioxus_rsx::BodyNode::Element(el) => el,
        _ => panic!("Expected an element"),
    };

    assert_eq!(el.name, "circle");

    assert_eq!(el.attributes.len(), 5);

    let mut context = DynamicContext::default();
    let o = context.render_static_node(&body.roots[0]);

    // hi!!!!!
    // you're probably here because you changed something in how rsx! generates templates and need to update the snapshot
    // This is a snapshot test. Make sure the contents are checked before committing a new snapshot.
    let stability_tested = o.to_string();
    assert_eq!(
        stability_tested.trim(),
        include_str!("./parsing/multiexpr.expanded.rsx").trim()
    );
}

fn parse_from_str(contents: &str) -> CallBody {
    // Parse the file
    let file = syn::parse_file(contents).unwrap();

    // The first token should be the macro call
    let Item::Macro(call) = file.items.first().unwrap() else {
        panic!("Expected a macro call");
    };

    call.mac.parse_body().unwrap()
}
