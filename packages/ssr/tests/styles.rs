use dioxus::prelude::*;

#[test]
fn static_styles() {
    fn app(cx: Scope) -> Element {
        render! { div { width: "100px" } }
    }

    let mut dom = VirtualDom::new(app);
    _ = dom.rebuild();

    assert_eq!(
        dioxus_ssr::render(&dom),
        r#"<div style="width:100px;"></div>"#
    );
}

#[test]
fn partially_dynamic_styles() {
    let dynamic = 123;

    assert_eq!(
        dioxus_ssr::render_lazy(rsx! {
            div { width: "100px", height: "{dynamic}px" }
        }),
        r#"<div style="width:100px;height:123px;"></div>"#
    );
}

#[test]
fn dynamic_styles() {
    let dynamic = 123;

    assert_eq!(
        dioxus_ssr::render_lazy(rsx! {
            div { width: "{dynamic}px" }
        }),
        r#"<div style="width:123px;"></div>"#
    );
}
