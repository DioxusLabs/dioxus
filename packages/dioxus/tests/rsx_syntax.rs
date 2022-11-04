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
    cx.render(rsx! {
        div {
            basic_child { }
            async_child { }
        }
    })
}

fn basic_child(cx: Scope) -> Element {
    todo!()
}

async fn async_child(cx: Scope<'_>) -> Element {
    todo!()
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

    // takes_it(basic_child);
}

// fn takes_it(f: fn(Scope) -> Element) {}
// fn takes_it(f: fn(Scope) -> Element) {}
