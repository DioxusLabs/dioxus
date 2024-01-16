// use dioxus::dioxus_core::Mutation::*;
use dioxus::prelude::*;

#[test]
fn multiroot() {
    let mut dom = VirtualDom::new(|| {
        rsx! {
            div { "Hello a" }
            div { "Hello b" }
            div { "Hello c" }
        }
    });

    // note: we dont test template edits anymore
    let _templates = dom.rebuild_to_vec().santize().templates;

    // assert_eq!(
    //     dom.rebuild_to_vec().santize().templates,
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
