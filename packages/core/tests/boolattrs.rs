use dioxus::dioxus_core::{ElementId, Mutation::*};
use dioxus::prelude::*;

#[test]
fn bool_test() {
    let mut app = VirtualDom::new(|| rsx!(div { hidden: false }));
    let _runtime_guard = RuntimeGuard::new(app.runtime());

    assert_eq!(
        app.rebuild_to_vec().edits,
        [
            LoadTemplate { index: 0, id: ElementId(1) },
            SetAttribute {
                name: "hidden",
                value: dioxus_core::AttributeValue::Bool(false),
                id: ElementId(1,),
                ns: None
            },
            AppendChildren { m: 1, id: ElementId(0) },
        ]
    );
}
