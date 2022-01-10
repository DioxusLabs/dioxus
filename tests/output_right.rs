use dioxus::prelude::*;

#[test]
fn bool_attrs() {
    let out = dioxus::ssr::render_lazy(rsx! { div { hidden: "true", } });
    assert_eq!(out, "<div hidden=\"true\"></div>");

    let out = dioxus::ssr::render_lazy(rsx! { div { hidden: "false", } });
    assert_eq!(out, "<div></div>");
}
