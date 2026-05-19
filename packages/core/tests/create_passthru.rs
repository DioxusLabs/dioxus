use dioxus::prelude::*;
use dioxus_renderer_oracle::Sequence;

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

    Sequence::new()
        .render_with_expected(app, rsx! { div { "hi" } })
        .run();
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

    Sequence::new()
        .render_with_expected(
            app,
            rsx! {
                "1"
                "2"
                "3"
                div { "hi" }
            },
        )
        .run();
}

/// note that the template is all dynamic roots - so it doesn't actually get cached as a template
#[test]
fn dynamic_node_as_root() {
    fn app() -> Element {
        let a = 123;
        let b = 456;
        rsx! { "{a}" "{b}" }
    }

    Sequence::new()
        .render_with_expected(app, rsx! { "123" "456" })
        .run();
}
