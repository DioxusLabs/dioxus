use dioxus::prelude::*;
use dioxus_renderer_oracle::RendererOracle;

fn basic_syntax_is_a_template() -> Element {
    let asd = 123;
    let var = 123;

    rsx! {
        div {
            key: "{asd}",
            class: "asd",
            class: "{asd}",
            class: if true { "{asd}" },
            class: if false { "{asd}" },
            onclick: move |_| {},
            div { "{var}" }
            div {
                h1 { "var" }
                p { "you're great!" }
                div { background_color: "red",
                    h1 { "var" }
                    div {
                        b { "asd" }
                        "not great"
                    }
                }
                p { "you're great!" }
            }
        }
    }
}

#[test]
fn dual_stream() {
    let mut dom = VirtualDom::new(basic_syntax_is_a_template);
    let mut oracle = RendererOracle::new();
    let summary = oracle.rebuild(&mut dom);

    oracle.assert_matches(basic_syntax_is_a_template);
    assert_eq!(summary.set_attrs, 1);
}
