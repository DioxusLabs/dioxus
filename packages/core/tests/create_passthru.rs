use dioxus::prelude::*;
use dioxus_renderer_oracle::RendererOracle;

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

    fn expected() -> Element {
        rsx! { div { "hi" } }
    }

    let mut dom = VirtualDom::new(app);
    let mut oracle = RendererOracle::new();
    oracle.rebuild(&mut dom);
    oracle.assert_matches(expected);
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

    fn expected() -> Element {
        rsx! {
            "1"
            "2"
            "3"
            div { "hi" }
        }
    }

    let mut dom = VirtualDom::new(app);
    let mut oracle = RendererOracle::new();
    oracle.rebuild(&mut dom);
    oracle.assert_matches(expected);
}

/// note that the template is all dynamic roots - so it doesn't actually get cached as a template
#[test]
fn dynamic_node_as_root() {
    fn app() -> Element {
        let a = 123;
        let b = 456;
        rsx! { "{a}" "{b}" }
    }

    fn expected() -> Element {
        rsx! { "123" "456" }
    }

    let mut dom = VirtualDom::new(app);
    let mut oracle = RendererOracle::new();
    oracle.rebuild(&mut dom);
    oracle.assert_matches(expected);
}
