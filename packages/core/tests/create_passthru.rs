use dioxus::dioxus_core::Mutation::*;
use dioxus::prelude::*;
use dioxus_core::ElementId;

/// Should push the text node onto the stack and modify it
#[test]
fn nested_passthru_creates() {
    fn app() -> Element {
        rsx! {
            PassThru {
                PassThru {
                    PassThru { div { "hi" } }
                }
            }
        }
    }

    #[component]
    fn PassThru(children: Element) -> Element {
        rsx!({ children })
    }

    let mut dom = VirtualDom::new(app);
    let edits = dom.rebuild_to_vec().santize();

    assert_eq!(
        edits.edits,
        [
            LoadTemplate { name: "template", index: 0, id: ElementId(1) },
            AppendChildren { m: 1, id: ElementId(0) },
        ]
    )
}

/// Should load all the templates and append them
///
/// Take note on how we don't spit out the template for child_comp since it's entirely dynamic
#[test]
fn nested_passthru_creates_add() {
    fn app() -> Element {
        rsx! {
            ChildComp {
                "1"
                ChildComp {
                    "2"
                    ChildComp {
                        "3"
                        div { "hi" }
                    }
                }
            }
        }
    }

    #[component]
    fn ChildComp(children: Element) -> Element {
        rsx! {{children}}
    }

    let mut dom = VirtualDom::new(app);

    assert_eq!(
        dom.rebuild_to_vec().santize().edits,
        [
            // load 1
            LoadTemplate { name: "template", index: 0, id: ElementId(1) },
            // load 2
            LoadTemplate { name: "template", index: 0, id: ElementId(2) },
            // load 3
            LoadTemplate { name: "template", index: 0, id: ElementId(3) },
            // load div that contains 4
            LoadTemplate { name: "template", index: 1, id: ElementId(4) },
            AppendChildren { id: ElementId(0), m: 4 },
        ]
    );
}

/// note that the template is all dynamic roots - so it doesn't actually get cached as a template
#[test]
fn dynamic_node_as_root() {
    fn app() -> Element {
        let a = 123;
        let b = 456;
        rsx! { "{a}", "{b}" }
    }

    let mut dom = VirtualDom::new(app);
    let edits = dom.rebuild_to_vec().santize();

    // Since the roots were all dynamic, they should not cause any template muations
    assert!(edits.templates.is_empty());

    // The root node is text, so we just create it on the spot
    assert_eq!(
        edits.edits,
        [
            CreateTextNode { value: "123".to_string(), id: ElementId(1) },
            CreateTextNode { value: "456".to_string(), id: ElementId(2) },
            AppendChildren { id: ElementId(0), m: 2 }
        ]
    )
}
