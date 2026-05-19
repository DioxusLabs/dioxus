use dioxus::prelude::*;
use dioxus_renderer_oracle::Sequence;

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
    Sequence::new()
        .render_with(basic_syntax_is_a_template)
        .assert_edit_summary(0, |s| assert_eq!(s.set_attrs, 1))
        .run();
}
