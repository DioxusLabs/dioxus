use dioxus::core::Mutation::*;
use dioxus::prelude::*;

#[test]
fn multiroot() {
    let mut dom = VirtualDom::new(|cx| {
        cx.render(rsx! {
            div { "Hello a" }
            div { "Hello b" }
            div { "Hello c" }
        })
    });

    assert_eq!(
        dom.rebuild().santize().template_edits,
        [
            CreateElement { name: "div" },
            CreateStaticText { value: "Hello a" },
            AppendChildren { m: 1 },
            CreateElement { name: "div" },
            CreateStaticText { value: "Hello b" },
            AppendChildren { m: 1 },
            CreateElement { name: "div" },
            CreateStaticText { value: "Hello c" },
            AppendChildren { m: 1 },
            SaveTemplate { name: "template", m: 3 }
        ]
    )
}
