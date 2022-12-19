use dioxus::core::{ElementId, Mutation::*};
use dioxus::prelude::*;

#[test]
fn bool_test() {
    let mut app = VirtualDom::new(|cx| cx.render(rsx!(div { hidden: false })));
    assert_eq!(
        app.rebuild().santize().edits,
        [
            LoadTemplate { name: "template", index: 0, id: ElementId(1) },
            SetBoolAttribute { name: "hidden", value: false, id: ElementId(1,) },
            AppendChildren { m: 1, id: ElementId(0) },
        ]
    )
}
