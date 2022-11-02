use dioxus::prelude::*;

fn basic_syntax_is_a_template(cx: Scope) -> Element {
    let asd = 123;
    let var = 123;

    cx.render(rsx! {
        div {
            class: "asd",
            class: "{asd}",
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
    })
}

fn basic_template(cx: Scope) -> Element {
    cx.render(rsx! {
        div {
            (0..2).map(|i| rsx! {
                div { "asd" }
            }),
            (0..2).map(|i| rsx! {
                div { "asd" }
            })
        }
    })
}

#[test]
fn basic_prints() {
    let mut dom = VirtualDom::new(basic_template);

    let mut edits = Vec::new();
    dom.rebuild(&mut edits);
    dbg!(edits);

    let mut edits = Vec::new();
    dom.rebuild(&mut edits);

    dbg!(edits);
    // let renderer = dioxus_edit_stream::Mutations::default();
    //
    // dbg!(renderer.edits);
}
