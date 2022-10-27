use dioxus::prelude::*;
use dioxus_core::{Attribute, TemplateAttribute};
use dioxus_edit_stream::*;

fn basic_syntax_is_a_template(cx: Scope) -> Element {
    let asd = 123;
    let var = 123;

    cx.render(rsx! {
        div { class: "asd", class: "{asd}",
            onclick: move |_| {},
            div { "{var}" }
            div {
                h1 { "var" }
                p { "you're great!" }
                div {
                    background_color: "red",
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
        div {"hi!"}
    })
}

#[test]
fn basic_prints() {
    let dom = VirtualDom::new(basic_template);

    let renderer = dioxus_edit_stream::Mutations::default();
    dom.rebuild(&mut renderer);

    dbg!(renderer.edits);
}
