use dioxus::prelude::*;

#[test]
fn escape_static_values() {
    fn app() -> Element {
        rsx! { input { disabled: "\"><div>" } }
    }

    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();

    assert_eq!(
        dioxus_ssr::render(&dom),
        "<input disabled=\"&#34;&#62;&#60;div&#62;\"/>"
    );
}

#[test]
fn escape_dynamic_values() {
    fn app() -> Element {
        let disabled = "\"><div>";
        rsx! { input { disabled } }
    }

    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();

    assert_eq!(
        dioxus_ssr::render(&dom),
        "<input disabled=\"&#34;&#62;&#60;div&#62;\"/>"
    );
}

#[test]
fn escape_static_style() {
    fn app() -> Element {
        rsx! { div { width: "\"><div>" } }
    }

    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();

    assert_eq!(
        dioxus_ssr::render(&dom),
        "<div style=\"width:&#34;&#62;&#60;div&#62;;\"></div>"
    );
}

#[test]
fn escape_dynamic_style() {
    fn app() -> Element {
        let width = "\"><div>";
        rsx! { div { width } }
    }

    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();

    assert_eq!(
        dioxus_ssr::render(&dom),
        "<div style=\"width:&#34;&#62;&#60;div&#62;;\"></div>"
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
    dom.rebuild_in_place();

    assert_eq!(
        dioxus_ssr::render(&dom),
        "<div>&#34;&#62;&#60;div&#62;</div>"
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
    dom.rebuild_in_place();

    assert_eq!(
        dioxus_ssr::render(&dom),
        "<div>&#34;&#62;&#60;div&#62;</div>"
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
    dom.rebuild_in_place();

    assert_eq!(
        dioxus_ssr::render(&dom),
        "<script>console.log('hello world');</script>"
    );
}

#[test]
fn don_t_escape_dynamic_scripts() {
    fn app() -> Element {
        // Named to avoid shadowing the `script` element: a value-namespace local
        // matching an element name would be picked up inside the const template.
        let script_text = "console.log('hello world');";
        rsx! {
            script {
                {script_text}
            }
        }
    }

    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();

    assert_eq!(
        dioxus_ssr::render(&dom),
        "<script>console.log('hello world');</script>"
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
    dom.rebuild_in_place();

    assert_eq!(
        dioxus_ssr::render(&dom),
        "<style>body { background-color: red; }</style>"
    );
}

#[test]
fn don_t_escape_dynamic_styles() {
    fn app() -> Element {
        // Named to avoid shadowing the `style` element (see `don_t_escape_dynamic_scripts`).
        let style_text = "body { font-family: \"sans-serif\"; }";
        rsx! {
            style {
                {style_text}
            }
        }
    }

    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();

    assert_eq!(
        dioxus_ssr::render(&dom),
        "<style>body { font-family: \"sans-serif\"; }</style>"
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
    dom.rebuild_in_place();

    assert_eq!(
        dioxus_ssr::render(&dom),
        "<style>body { font-family: \"sans-serif\"; }</style>"
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
    dom.rebuild_in_place();

    assert_eq!(
        dioxus_ssr::render(&dom),
        "<div>body { font-family: &#34;sans-serif&#34;; }</div>"
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
    dom.rebuild_in_place();

    assert_eq!(
        dioxus_ssr::render(&dom),
        "<div>body { font-family: &#34;sans-serif&#34;; }</div>"
    );
}
