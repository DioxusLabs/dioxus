use dioxus::prelude::*;

#[test]
fn static_inner_html() {
    fn app(_: ()) -> Element {
        render! { div { dangerous_inner_html: "<div>1234</div>" } }
    }

    let mut dom = VirtualDom::new(app);
    _ = dom.rebuild(&mut dioxus_core::NoOpMutations);

    assert_eq!(dioxus_ssr::render(&dom), r#"<div><div>1234</div></div>"#);
}

#[test]
fn dynamic_inner_html() {
    fn app(_: ()) -> Element {
        let inner_html = "<div>1234</div>";
        render! { div { dangerous_inner_html: "{inner_html}" } }
    }

    let mut dom = VirtualDom::new(app);
    _ = dom.rebuild(&mut dioxus_core::NoOpMutations);

    assert_eq!(dioxus_ssr::render(&dom), r#"<div><div>1234</div></div>"#);
}
