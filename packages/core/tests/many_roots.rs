use dioxus::dioxus_core::Mutation::*;
use dioxus::prelude::*;
use dioxus_core::ElementId;

/// Should push the text node onto the stack and modify it
#[test]
fn many_roots() {
    fn app() -> Element {
        rsx! {
            div {
                MyNav {}
                MyOutlet {}
            }
            MyFooter {}
        }
    }

    fn MyFooter() -> Element {
        rsx! {
            div { "footer" }
        }
    }

    fn MyNav() -> Element {
        rsx!(
            div { "trailing nav" }
            MySearch {}
        )
    }

    fn MySearch() -> Element {
        rsx!("search")
    }

    fn MyOutlet() -> Element {
        rsx!(
            if true {
                div {
                    "homepage"
                }
            }
        )
    }

    let mut dom = VirtualDom::new(app);
    let edits = dom.rebuild_to_vec();

    println!("{:#?}", edits.edits);

    // assert_eq!(
    //     edits.edits,
    //     [
    //         LoadTemplate { index: 0, id: ElementId(1) },
    //         AppendChildren { m: 1, id: ElementId(0) },
    //     ]
    // )
}
