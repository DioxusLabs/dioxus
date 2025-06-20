use dioxus::prelude::*;

#[test]
fn escape_static_values() {
    fn app() -> Element {
        rsx! { input { disabled: "\"><div>" } }
    }

    let mut dom = VirtualDom::new(app);
    dom.rebuild(&mut dioxus_core::NoOpMutations);

    assert_eq!(
        dioxus_ssr::pre_render(&dom),
        "<input disabled=\"&#34;&#62;&#60;div&#62;\" data-node-hydration=\"0\"/>"
    );
}

#[test]
fn escape_dynamic_values() {
    fn app() -> Element {
        let disabled = "\"><div>";
        rsx! { input { disabled } }
    }

    let mut dom = VirtualDom::new(app);
    dom.rebuild(&mut dioxus_core::NoOpMutations);

    assert_eq!(
        dioxus_ssr::pre_render(&dom),
        "<input disabled=\"&#34;&#62;&#60;div&#62;\" data-node-hydration=\"0\"/>"
    );
}

#[test]
fn escape_static_style() {
    fn app() -> Element {
        rsx! { div { width: "\"><div>" } }
    }

    let mut dom = VirtualDom::new(app);
    dom.rebuild(&mut dioxus_core::NoOpMutations);

    assert_eq!(
        dioxus_ssr::pre_render(&dom),
        "<div style=\"width:&#34;&#62;&#60;div&#62;;\" data-node-hydration=\"0\"></div>"
    );
}

#[test]
fn escape_dynamic_style() {
    fn app() -> Element {
        let width = "\"><div>";
        rsx! { div { width } }
    }

    let mut dom = VirtualDom::new(app);
    dom.rebuild(&mut dioxus_core::NoOpMutations);

    assert_eq!(
        dioxus_ssr::pre_render(&dom),
        "<div style=\"width:&#34;&#62;&#60;div&#62;;\" data-node-hydration=\"0\"></div>"
    );
}

#[test]
fn escape_static_text() {
    fn app() -> Element {
        rsx! {
            div {
                "\"><div>"
            }
        }
    }

    let mut dom = VirtualDom::new(app);
    dom.rebuild(&mut dioxus_core::NoOpMutations);

    assert_eq!(
        dioxus_ssr::pre_render(&dom),
        "<div data-node-hydration=\"0\">&#34;&#62;&#60;div&#62;</div>"
    );
}

#[test]
fn escape_dynamic_text() {
    fn app() -> Element {
        let text = "\"><div>";
        rsx! {
            div {
                {text}
            }
        }
    }

    let mut dom = VirtualDom::new(app);
    dom.rebuild(&mut dioxus_core::NoOpMutations);

    assert_eq!(
        dioxus_ssr::pre_render(&dom),
        "<div data-node-hydration=\"0\"><!--node-id1-->&#34;&#62;&#60;div&#62;<!--#--></div>"
    );
}

#[test]
fn don_t_escape_static_scripts() {
    fn app() -> Element {
        rsx! {
            script {
                "console.log('hello world');"
            }
        }
    }

    let mut dom = VirtualDom::new(app);
    dom.rebuild(&mut dioxus_core::NoOpMutations);

    assert_eq!(
        dioxus_ssr::pre_render(&dom),
        "<script data-node-hydration=\"0\">console.log('hello world');</script>"
    );
}

#[test]
fn don_t_escape_dynamic_scripts() {
    fn app() -> Element {
        let script = "console.log('hello world');";
        rsx! {
            script {
                {script}
            }
        }
    }

    let mut dom = VirtualDom::new(app);
    dom.rebuild(&mut dioxus_core::NoOpMutations);

    assert_eq!(
        dioxus_ssr::pre_render(&dom),
        "<script data-node-hydration=\"0\"><!--node-id1-->console.log('hello world');<!--#--></script>"
    );
}

#[test]
fn don_t_escape_static_styles() {
    fn app() -> Element {
        rsx! {
            style {
                "body {{ background-color: red; }}"
            }
        }
    }

    let mut dom = VirtualDom::new(app);
    dom.rebuild(&mut dioxus_core::NoOpMutations);

    assert_eq!(
        dioxus_ssr::pre_render(&dom),
        "<style data-node-hydration=\"0\">body { background-color: red; }</style>"
    );
}

#[test]
fn don_t_escape_dynamic_styles() {
    fn app() -> Element {
        let style = "body { font-family: \"sans-serif\"; }";
        rsx! {
            style {
                {style}
            }
        }
    }

    let mut dom = VirtualDom::new(app);
    dom.rebuild(&mut dioxus_core::NoOpMutations);

    assert_eq!(
        dioxus_ssr::pre_render(&dom),
        "<style data-node-hydration=\"0\"><!--node-id1-->body { font-family: \"sans-serif\"; }<!--#--></style>"
    );
}

#[test]
fn don_t_escape_static_fragment_styles() {
    fn app() -> Element {
        let style_element = rsx! { "body {{ font-family: \"sans-serif\"; }}" };
        rsx! {
            style {
                {style_element}
            }
        }
    }

    let mut dom = VirtualDom::new(app);
    dom.rebuild(&mut dioxus_core::NoOpMutations);

    assert_eq!(
        dioxus_ssr::pre_render(&dom),
        "<style data-node-hydration=\"0\"><!--node-id1-->body { font-family: \"sans-serif\"; }<!--#--></style>"
    );
}

#[test]
fn escape_static_component_fragment_div() {
    #[component]
    fn StyleContents() -> Element {
        rsx! { "body {{ font-family: \"sans-serif\"; }}" }
    }

    fn app() -> Element {
        rsx! {
            div {
                StyleContents {}
            }
        }
    }

    let mut dom = VirtualDom::new(app);
    dom.rebuild(&mut dioxus_core::NoOpMutations);

    assert_eq!(
        dioxus_ssr::pre_render(&dom),
        "<div data-node-hydration=\"0\"><!--node-id1-->body { font-family: &#34;sans-serif&#34;; }<!--#--></div>"
    );
}

#[test]
fn escape_dynamic_component_fragment_div() {
    #[component]
    fn StyleContents() -> Element {
        let dynamic = "body { font-family: \"sans-serif\"; }";
        rsx! { "{dynamic}" }
    }

    fn app() -> Element {
        rsx! {
            div {
                StyleContents {}
            }
        }
    }

    let mut dom = VirtualDom::new(app);
    dom.rebuild(&mut dioxus_core::NoOpMutations);

    assert_eq!(
        dioxus_ssr::pre_render(&dom),
        "<div data-node-hydration=\"0\"><!--node-id1-->body { font-family: &#34;sans-serif&#34;; }<!--#--></div>"
    );
}
