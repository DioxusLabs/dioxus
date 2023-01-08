use bumpalo::Bump;
use dioxus::core::{ElementId, Mutation::*};
use dioxus::prelude::*;

#[test]
fn bool_test() {
    let mut app = VirtualDom::new(|cx| cx.render(rsx!(div { hidden: false })));
    let bump = Bump::new();

    assert_eq!(
        app.rebuild().santize().edits,
        [
            LoadTemplate { name: "template", index: 0, id: ElementId(1) },
            SetAttribute {
                name: "hidden",
                value: (&*bump.alloc(false.into_value(&bump))).into(),
                id: ElementId(1,),
                ns: None
            },
            AppendChildren { m: 1, id: ElementId(0) },
        ]
    );
}
