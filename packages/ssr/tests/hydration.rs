use dioxus::prelude::*;

#[test]
fn root_ids() {
    fn app() -> Element {
        rsx! { div { width: "100px" } }
    }

    let mut dom = VirtualDom::new(app);
    dom.rebuild(&mut dioxus_core::NoOpMutations);

    assert_eq!(
        dioxus_ssr::pre_render(&dom),
        r#"<div style="width:100px;" data-node-hydration="0"></div>"#
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
        dioxus_ssr::pre_render(&dom),
        r#"<div style="width:100px;" data-node-hydration="0"><div style="width:123px;" data-node-hydration="1"></div></div>"#
    );
}

#[test]
fn listeners() {
    fn app() -> Element {
        rsx! {
            div { width: "100px", div { onclick: |_| {} } }
        }
    }

    let mut dom = VirtualDom::new(app);
    dom.rebuild(&mut dioxus_core::NoOpMutations);

    assert_eq!(
        dioxus_ssr::pre_render(&dom),
        r#"<div style="width:100px;" data-node-hydration="0"><div data-node-hydration="1,click:1"></div></div>"#
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
        dioxus_ssr::pre_render(&dom),
        r#"<div style="width:100px;" data-node-hydration="0"><div style="width:123px;" data-node-hydration="1,click:1"></div></div>"#
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

    assert_eq!(
        dioxus_ssr::pre_render(&dom),
        r#"<div data-node-hydration="0"><!--node-id1-->hello<!--#--></div>"#
    );

    fn app2() -> Element {
        let dynamic = 123;
        rsx! {
            div { "{dynamic}", "{1234}" }
        }
    }

    let mut dom = VirtualDom::new(app2);
    dom.rebuild(&mut dioxus_core::NoOpMutations);

    assert_eq!(
        dioxus_ssr::pre_render(&dom),
        r#"<div data-node-hydration="0"><!--node-id1-->123<!--#--><!--node-id2-->1234<!--#--></div>"#
    );
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

    assert_eq!(
        dioxus_ssr::pre_render(&dom),
        r#"<div data-node-hydration="0">hello</div>"#
    );

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

    assert_eq!(
        dioxus_ssr::pre_render(&dom),
        r#"<div data-node-hydration="0"><!--node-id1-->hello<!--#--></div>"#
    );

    fn app3() -> Element {
        rsx! { Child3 {} }
    }

    fn Child3() -> Element {
        rsx! { div { width: "{1}" } }
    }

    let mut dom = VirtualDom::new(app3);
    dom.rebuild(&mut dioxus_core::NoOpMutations);

    assert_eq!(
        dioxus_ssr::pre_render(&dom),
        r#"<div style="width:1;" data-node-hydration="0"></div>"#
    );

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

    assert_eq!(
        dioxus_ssr::pre_render(&dom),
        r#"<!--node-id0-->1<!--#--><!--node-id1-->1<!--#-->"#
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
    dom.rebuild(&mut dioxus_core::NoOpMutations);

    assert_eq!(
        dioxus_ssr::pre_render(&dom),
        r#"<h1 data-node-hydration="0"><!--node-id1-->High-Five counter: 0<!--#--></h1><button data-node-hydration="2,click:1">Up high!</button><button data-node-hydration="3,click:1">Down low!</button>"#
    );
}
