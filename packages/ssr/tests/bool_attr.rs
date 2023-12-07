use dioxus::prelude::*;

#[test]
fn static_boolean_attributs() {
    fn app(cx: Scope) -> Element {
        render! {
            div { hidden: "false" }
            div { hidden: "true" }
        }
    }

    let mut dom = VirtualDom::new(app);
    _ = dom.rebuild();

    assert_eq!(
        dioxus_ssr::render(&dom),
        r#"<div></div><div hidden="true"></div>"#
    );
}

#[test]
fn dynamic_boolean_attributs() {
    fn app(cx: Scope) -> Element {
        render! {
            div { hidden: false }
            div { hidden: true }
        }
    }

    let mut dom = VirtualDom::new(app);
    _ = dom.rebuild();

    assert_eq!(
        dioxus_ssr::render(&dom),
        r#"<div></div><div hidden=true></div>"#
    );
}
