use dioxus::prelude::*;

fn basic_syntax_is_a_template(cx: Scope) -> Element {
    let asd = 123;
    let var = 123;

    cx.render(rsx! {
        div { key: "12345",
            class: "asd",
            class: "{asd}",
            onclick: move |_| {},
            div { "{var}" }
            div {
                h1 { "var" }
                p { "you're great!" }
                div { background_color: "red",
                    h1 { "var" }
                    div { b { "asd" } "not great" }
                }
                p { "you're great!" }
            }
        }
    })
}

#[test]
fn dual_stream() {
    let mut dom = VirtualDom::new(basic_syntax_is_a_template);
    dbg!(dom.rebuild());
}
