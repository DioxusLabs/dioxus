use dioxus::prelude::*;

#[test]
fn root_ids() {
    fn app() -> Element {
        rsx! { div { width: "100px" } }
    }

    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();

    assert_eq!(
        dioxus_ssr::render(&dom),
        r#"<div style="width:100px;"></div>"#
    );
}

#[test]
fn dynamic_attributes() {
    fn app() -> Element {
        let dynamic = 123;
        rsx! {
            div { width: "100px", div { width: "{dynamic}px" } }
        }
    }

    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();

    assert_eq!(
        dioxus_ssr::render(&dom),
        r#"<div style="width:100px;"><div style="width:123px;"></div></div>"#
    );
}

#[test]
fn listeners() {
    // Listeners are attached on the client by the walk script — they leave no
    // trace in the SSR HTML.
    fn app() -> Element {
        rsx! {
            div { width: "100px", div { onclick: |_| {} } }
        }
    }

    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();

    assert_eq!(
        dioxus_ssr::render(&dom),
        r#"<div style="width:100px;"><div></div></div>"#
    );

    fn app2() -> Element {
        let dynamic = 123;
        rsx! {
            div { width: "100px", div { width: "{dynamic}px", onclick: |_| {} } }
        }
    }

    let mut dom = VirtualDom::new(app2);
    dom.rebuild_in_place();

    assert_eq!(
        dioxus_ssr::render(&dom),
        r#"<div style="width:100px;"><div style="width:123px;"></div></div>"#
    );
}

#[test]
fn text_nodes() {
    fn app() -> Element {
        let dynamic_text = "hello";
        rsx! {
            div { {dynamic_text} }
        }
    }

    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();

    assert_eq!(dioxus_ssr::render(&dom), r#"<div>hello</div>"#);

    fn app2() -> Element {
        let dynamic = 123;
        rsx! {
            div { "{dynamic}" "{1234}" }
        }
    }

    let mut dom = VirtualDom::new(app2);
    dom.rebuild_in_place();

    // Adjacent dynamic texts merge into a single DOM text node — hydration splits
    // them apart at known offsets via `SplitText` rather than relying on parser
    // boundary markers.
    assert_eq!(dioxus_ssr::render(&dom), r#"<div>1231234</div>"#);
}

#[allow(non_snake_case)]
#[test]
fn components_hydrate() {
    fn app() -> Element {
        rsx! { Child {} }
    }

    fn Child() -> Element {
        rsx! { div { "hello" } }
    }

    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();

    assert_eq!(dioxus_ssr::render(&dom), r#"<div>hello</div>"#);

    fn app2() -> Element {
        rsx! { Child2 {} }
    }

    fn Child2() -> Element {
        let dyn_text = "hello";
        rsx! {
            div { {dyn_text} }
        }
    }

    let mut dom = VirtualDom::new(app2);
    dom.rebuild_in_place();

    assert_eq!(dioxus_ssr::render(&dom), r#"<div>hello</div>"#);

    fn app3() -> Element {
        rsx! { Child3 {} }
    }

    fn Child3() -> Element {
        rsx! { div { width: "{1}" } }
    }

    let mut dom = VirtualDom::new(app3);
    dom.rebuild_in_place();

    assert_eq!(dioxus_ssr::render(&dom), r#"<div style="width:1;"></div>"#);

    fn app4() -> Element {
        rsx! { Child4 {} }
    }

    fn Child4() -> Element {
        rsx! {
            for _ in 0..2 {
                {rsx! { "{1}" }}
            }
        }
    }

    let mut dom = VirtualDom::new(app4);
    dom.rebuild_in_place();

    assert_eq!(dioxus_ssr::render(&dom), r#"11"#);
}

#[test]
fn textarea_children_render_without_markers() {
    // Regression test for https://github.com/DioxusLabs/dioxus/issues/5548.
    // `textarea` interprets its children as raw text, so the SSR output must
    // contain no hydration markers around the dynamic text — the markerless walk
    // reconstructs the dynamic-text position on the client. With the old
    // comment-marker hydration this rendered stray markers inside the textarea.
    fn app() -> Element {
        let value = "hello world";
        rsx! {
            textarea { "{value}" }
        }
    }

    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();

    assert_eq!(
        dioxus_ssr::render(&dom),
        r#"<textarea>hello world</textarea>"#
    );

    // A static prefix immediately followed by dynamic text still emits no marker
    // between the two contributions.
    fn app2() -> Element {
        let value = "world";
        rsx! {
            textarea { "hello " "{value}" }
        }
    }

    let mut dom = VirtualDom::new(app2);
    dom.rebuild_in_place();

    assert_eq!(
        dioxus_ssr::render(&dom),
        r#"<textarea>hello world</textarea>"#
    );
}

#[test]
fn hello_world_hydrates() {
    use dioxus::hooks::use_signal;

    fn app() -> Element {
        let mut count = use_signal(|| 0);

        rsx! {
            h1 { "High-Five counter: {count}" }
            button { onclick: move |_| count += 1, "Up high!" }
            button { onclick: move |_| count -= 1, "Down low!" }
        }
    }

    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();

    assert_eq!(
        dioxus_ssr::render(&dom),
        r#"<h1>High-Five counter: 0</h1><button>Up high!</button><button>Down low!</button>"#
    );
}
