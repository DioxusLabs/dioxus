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

fn basic_template(cx: Scope) -> Element {
    let val = 123;

    cx.component(basic_child, (), "fn_name");

    todo!()
    // cx.render(rsx! {
    // div { class: "{val}", class: "{val}", class: "{val}", class: "{val}",
    // (0..2).map(|i| rsx! { div { "asd {i}" } })
    // basic_child { }
    // }
    // })
}

/// A beautiful component
fn basic_child(cx: Scope) -> Element {
    todo!()
}

async fn async_component(cx: Scope<'_>) -> Element {
    cx.render(rsx! {
        div { class: "asd" }
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
