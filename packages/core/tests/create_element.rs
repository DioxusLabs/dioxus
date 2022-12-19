// use dioxus::core::Mutation::*;
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

    // note: we dont test template edits anymore
    let _templates = dom.rebuild().santize().templates;

    // assert_eq!(
    //     dom.rebuild().santize().templates,
    //     [
    //         CreateElement { name: "div" },
    //         CreateStaticText { value: "Hello a" },
    //         AppendChildren { m: 1 },
    //         CreateElement { name: "div" },
    //         CreateStaticText { value: "Hello b" },
    //         AppendChildren { m: 1 },
    //         CreateElement { name: "div" },
    //         CreateStaticText { value: "Hello c" },
    //         AppendChildren { m: 1 },
    //         SaveTemplate { name: "template", m: 3 }
    //     ]
    // )
}
