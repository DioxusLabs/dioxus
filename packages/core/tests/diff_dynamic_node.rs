use dioxus::prelude::*;
use dioxus_core::generation;
use dioxus_renderer_oracle::Sequence;

#[test]
fn toggle_option_text() {
    fn empty() -> Element {
        let text: Option<&str> = None;
        rsx! {
            div {
                {text}
            }
        }
    }

    Sequence::new()
        .render_with_expected(empty, rsx! { div {} })
        .render(rsx! { div { "hello" } })
        .render_with_expected(empty, rsx! { div {} })
        .assert_edit_summary(1, |s| assert_eq!(s.replaces, 1))
        .assert_edit_summary(2, |s| assert_eq!(s.replaces, 1))
        .run();
}

// Regression test for https://github.com/DioxusLabs/dioxus/issues/2815
#[test]
fn toggle_template() {
    fn app() -> Element {
        rsx!(
            Comp {
                if true {
                    "{true}"
                }
            }
        )
    }

    #[component]
    fn Comp(children: Element) -> Element {
        let show = generation() % 2 == 0;

        rsx! {
            if show {
                {children}
            }
        }
    }

    Sequence::new()
        .render_with_expected(app, rsx! { "true" })
        .render_with_expected(app, rsx!({}))
        .render_with_expected(app, rsx! { "true" })
        .render_with_expected(app, rsx!({}))
        .render_with_expected(app, rsx! { "true" })
        .assert_edit_summary(1, |s| assert_eq!(s.replaces, 1))
        .assert_edit_summary(2, |s| assert_eq!(s.replaces, 1))
        .assert_edit_summary(3, |s| assert_eq!(s.replaces, 1))
        .assert_edit_summary(4, |s| assert_eq!(s.replaces, 1))
        .run();
}
