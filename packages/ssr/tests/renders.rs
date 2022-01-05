use dioxus_core::prelude::*;
use dioxus_core_macro::*;
use dioxus_html as dioxus_elements;
use dioxus_ssr::{render_lazy, render_vdom, render_vdom_cfg, SsrConfig, SsrRenderer, TextRenderer};

static SIMPLE_APP: Component = |cx| {
    cx.render(rsx!(div {
        "hello world!"
    }))
};

static SLIGHTLY_MORE_COMPLEX: Component = |cx| {
    cx.render(rsx! {
        div { title: "About W3Schools",
            (0..20).map(|f| rsx!{
                div {
                    title: "About W3Schools",
                    style: "color:blue;text-align:center",
                    class: "About W3Schools",
                    p {
                        title: "About W3Schools",
                        "Hello world!: {f}"
                    }
                }
            })
        }
    })
};

static NESTED_APP: Component = |cx| {
    cx.render(rsx!(
        div {
            SIMPLE_APP {}
        }
    ))
};
static FRAGMENT_APP: Component = |cx| {
    cx.render(rsx!(
        div { "f1" }
        div { "f2" }
        div { "f3" }
        div { "f4" }
    ))
};

#[test]
fn to_string_works() {
    let mut dom = VirtualDom::new(SIMPLE_APP);
    dom.rebuild();
    dbg!(render_vdom(&dom));
}

#[test]
fn hydration() {
    let mut dom = VirtualDom::new(NESTED_APP);
    dom.rebuild();
    dbg!(render_vdom_cfg(&dom, |c| c.pre_render(true)));
}

#[test]
fn nested() {
    let mut dom = VirtualDom::new(NESTED_APP);
    dom.rebuild();
    dbg!(render_vdom(&dom));
}

#[test]
fn fragment_app() {
    let mut dom = VirtualDom::new(FRAGMENT_APP);
    dom.rebuild();
    dbg!(render_vdom(&dom));
}

#[test]
fn write_to_file() {
    use std::fs::File;
    use std::io::Write;

    let mut file = File::create("index.html").unwrap();

    let mut dom = VirtualDom::new(SLIGHTLY_MORE_COMPLEX);
    dom.rebuild();

    file.write_fmt(format_args!(
        "{}",
        TextRenderer::from_vdom(&dom, SsrConfig::default())
    ))
    .unwrap();
}

#[test]
fn styles() {
    static STLYE_APP: Component = |cx| {
        cx.render(rsx! {
            div { color: "blue", font_size: "46px"  }
        })
    };

    let mut dom = VirtualDom::new(STLYE_APP);
    dom.rebuild();
    dbg!(render_vdom(&dom));
}

#[test]
fn lazy() {
    let p1 = SsrRenderer::new(|c| c).render_lazy(rsx! {
        div { "ello"  }
    });

    let p2 = render_lazy(rsx! {
        div {
            "ello"
        }
    });
    assert_eq!(p1, p2);
}

#[test]
fn big_lazy() {
    let s = render_lazy(rsx! {
        div {
            div {
                div {
                    h1 { "ello world" }
                    h1 { "ello world" }
                    h1 { "ello world" }
                    h1 { "ello world" }
                    h1 { "ello world" }
                }
            }
        }
    });

    dbg!(s);
}

#[test]
fn inner_html() {
    let s = render_lazy(rsx! {
        div {
            dangerous_inner_html: "<div> ack </div>"
        }
    });

    dbg!(s);
}
