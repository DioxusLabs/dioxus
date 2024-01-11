use dioxus::core::Mutation::*;
use dioxus::prelude::*;
use dioxus_core::ElementId;

/// Should push the text node onto the stack and modify it
#[test]
fn nested_passthru_creates() {
    #[component]
    fn App(cx: Scope) -> Element {
        cx.render(rsx! {
            PassThru {
                PassThru {
                    PassThru {
                        div { "hi" }
                    }
                }
            }
        })
    }

    #[component]
    fn PassThru<'a>(cx: Scope<'a>, children: Element<'a>) -> Element {
        cx.render(rsx!({ children }))
    }

    let mut dom = VirtualDom::new(App);
    let edits = dom.rebuild().santize();

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
    #[component]
    fn App(cx: Scope) -> Element {
        cx.render(rsx! {
            ChildComp {
                "1"
                ChildComp {
                    "2"
                    ChildComp {
                        "3"
                        div {
                            "hi"
                        }
                    }
                }
            }
        })
    }

    #[component]
    fn ChildComp<'a>(cx: Scope, children: Element<'a>) -> Element {
        cx.render(rsx! { {children} })
    }

    let mut dom = VirtualDom::new(App);

    assert_eq!(
        dom.rebuild().santize().edits,
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
    #[component]
    fn App(cx: Scope) -> Element {
        let a = 123;
        let b = 456;
        cx.render(rsx! { "{a}" "{b}" })
    }

    let mut dom = VirtualDom::new(App);
    let edits = dom.rebuild().santize();

    // Since the roots were all dynamic, they should not cause any template muations
    assert!(edits.templates.is_empty());

    // The root node is text, so we just create it on the spot
    assert_eq!(
        edits.edits,
        [
            CreateTextNode { value: "123", id: ElementId(1) },
            CreateTextNode { value: "456", id: ElementId(2) },
            AppendChildren { id: ElementId(0), m: 2 }
        ]
    )
}
