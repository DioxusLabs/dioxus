fn component(cx: Scope) -> Element {
    cx.render(rsx! {
        div { hidden: false }
    })
}

use dioxus::prelude::*;

#[test]
fn bool_test() {
    let mut app = VirtualDom::new(component);
    let edits = app.rebuild();

    use dioxus_core::{ElementId, Mutation::*};
    assert_eq!(
        edits.edits,
        vec![
            LoadTemplate { name: "packages/dioxus/tests/boolattrs.rs:2:15:66", index: 0 },
            AssignId { path: &[], id: ElementId(2,) },
            SetBoolAttribute { name: "hidden", value: false, id: ElementId(2,) },
            AppendChildren { m: 1 },
        ]
    )
}
