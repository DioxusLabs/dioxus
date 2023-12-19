use dioxus::prelude::*;

#[test]
fn root_ids() {
    fn app(cx: Scope) -> Element {
        render! { div { width: "100px" } }
    }

    let mut dom = VirtualDom::new(app);
    _ = dom.rebuild();

    assert_eq!(
        dioxus_ssr::pre_render(&dom),
        r#"<div style="width:100px;" data-node-hydration="1"></div>"#
    );
}

#[test]
fn dynamic_attributes() {
    fn app(cx: Scope) -> Element {
        let dynamic = 123;
        render! {
            div { width: "100px", div { width: "{dynamic}px" } }
        }
    }

    let mut dom = VirtualDom::new(app);
    _ = dom.rebuild();

    assert_eq!(
        dioxus_ssr::pre_render(&dom),
        r#"<div style="width:100px;" data-node-hydration="1"><div style="width:123px;" data-node-hydration="2"></div></div>"#
    );
}

#[test]
fn listeners() {
    fn app(cx: Scope) -> Element {
        render! {
            div { width: "100px", div { onclick: |_| {} } }
        }
    }

    let mut dom = VirtualDom::new(app);
    _ = dom.rebuild();

    assert_eq!(
        dioxus_ssr::pre_render(&dom),
        r#"<div style="width:100px;" data-node-hydration="1"><div data-node-hydration="2,click:1"></div></div>"#
    );

    fn app2(cx: Scope) -> Element {
        let dynamic = 123;
        render! {
            div { width: "100px", div { width: "{dynamic}px", onclick: |_| {} } }
        }
    }

    let mut dom = VirtualDom::new(app2);
    _ = dom.rebuild();

    assert_eq!(
        dioxus_ssr::pre_render(&dom),
        r#"<div style="width:100px;" data-node-hydration="1"><div style="width:123px;" data-node-hydration="2,click:1"></div></div>"#
    );
}

#[test]
fn text_nodes() {
    fn app(cx: Scope) -> Element {
        let dynamic_text = "hello";
        render! {
            div { dynamic_text }
        }
    }

    let mut dom = VirtualDom::new(app);
    _ = dom.rebuild();

    assert_eq!(
        dioxus_ssr::pre_render(&dom),
        r#"<div data-node-hydration="1"><!--node-id2-->hello<!--#--></div>"#
    );

    fn app2(cx: Scope) -> Element {
        let dynamic = 123;
        render! {
            div { "{dynamic}", "{1234}" }
        }
    }

    let mut dom = VirtualDom::new(app2);
    _ = dom.rebuild();

    assert_eq!(
        dioxus_ssr::pre_render(&dom),
        r#"<div data-node-hydration="1"><!--node-id3-->123<!--#--><!--node-id2-->1234<!--#--></div>"#
    );
}

#[test]
fn hello_world_hydrates() {
    fn app(cx: Scope) -> Element {
        let mut count = use_state(cx, || 0);

        cx.render(rsx! {
            h1 { "High-Five counter: {count}" }
            button { onclick: move |_| count += 1, "Up high!" }
            button { onclick: move |_| count -= 1, "Down low!" }
        })
    }

    let mut dom = VirtualDom::new(app);
    _ = dbg!(dom.rebuild());

    assert_eq!(
        dioxus_ssr::pre_render(&dom),
        r#"<h1 data-node-hydration="1"><!--node-id2-->High-Five counter: 0<!--#--></h1><button data-node-hydration="3,click:1">Up high!</button><button data-node-hydration="4,click:1">Down low!</button>"#
    );
}
