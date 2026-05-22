use dioxus::prelude::*;

#[test]
fn root_ids() {
    fn app() -> Element {
        rsx! { div { width: "100px" } }
    }

    let mut dom = VirtualDom::new(app);
    dom.rebuild(&mut dioxus_core::NoOpMutations);

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
    dom.rebuild(&mut dioxus_core::NoOpMutations);

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
    dom.rebuild(&mut dioxus_core::NoOpMutations);

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
    dom.rebuild(&mut dioxus_core::NoOpMutations);

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
    dom.rebuild(&mut dioxus_core::NoOpMutations);

    assert_eq!(dioxus_ssr::render(&dom), r#"<div>hello</div>"#);

    fn app2() -> Element {
        let dynamic = 123;
        rsx! {
            div { "{dynamic}" "{1234}" }
        }
    }

    let mut dom = VirtualDom::new(app2);
    dom.rebuild(&mut dioxus_core::NoOpMutations);

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
    dom.rebuild(&mut dioxus_core::NoOpMutations);

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
    dom.rebuild(&mut dioxus_core::NoOpMutations);

    assert_eq!(dioxus_ssr::render(&dom), r#"<div>hello</div>"#);

    fn app3() -> Element {
        rsx! { Child3 {} }
    }

    fn Child3() -> Element {
        rsx! { div { width: "{1}" } }
    }

    let mut dom = VirtualDom::new(app3);
    dom.rebuild(&mut dioxus_core::NoOpMutations);

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
    dom.rebuild(&mut dioxus_core::NoOpMutations);

    assert_eq!(dioxus_ssr::render(&dom), r#"11"#);
}

// Regression test for https://github.com/DioxusLabs/components/issues/202
// In the old comment-based hydration scheme, `<!--placeholder0-->` inside a
// `<textarea>` was parsed as literal text by the browser. With markerless
// hydration there are no comments anywhere, so this entire class of bugs is gone.
#[test]
fn raw_text_elements_have_no_hydration_artifacts() {
    fn textarea_with_placeholder() -> Element {
        let children: Element = rsx! {};
        rsx! {
            textarea { value: "abc", {children} }
        }
    }

    let mut dom = VirtualDom::new(textarea_with_placeholder);
    dom.rebuild(&mut dioxus_core::NoOpMutations);

    let rendered = dioxus_ssr::render(&dom);
    assert!(
        !rendered.contains("<!--"),
        "no comments in markerless hydration output, got: {rendered}"
    );
    assert!(
        !rendered.contains("data-node-hydration"),
        "no hydration attributes either, got: {rendered}"
    );

    fn textarea_with_dynamic_text() -> Element {
        let value = "hello & world";
        rsx! {
            textarea { "{value}" }
        }
    }

    let mut dom = VirtualDom::new(textarea_with_dynamic_text);
    dom.rebuild(&mut dioxus_core::NoOpMutations);

    let rendered = dioxus_ssr::render(&dom);
    assert!(!rendered.contains("<!--"));
    assert!(rendered.contains("hello &#38; world"));
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
    dom.rebuild(&mut dioxus_core::NoOpMutations);

    assert_eq!(
        dioxus_ssr::render(&dom),
        r#"<h1>High-Five counter: 0</h1><button>Up high!</button><button>Down low!</button>"#
    );
}
