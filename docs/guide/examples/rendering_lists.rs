#![allow(non_snake_case)]

use dioxus::prelude::*;

fn main() {
    dioxus::desktop::launch(App);
}

pub fn App(cx: Scope) -> Element {
    // ANCHOR: render_list
    let comment_field = use_state(&cx, || String::new());
    let comments = use_ref(&cx, || Vec::<String>::new());

    let comments_lock = comments.read();
    let comments_rendered = comments_lock.iter().map(|comment| {
        cx.render(rsx!(
            div {
                "Comment by anon:",
                p { "{comment}" }
                button { "Reply" },
            }
        ))
    });

    cx.render(rsx!(
        form {
            onsubmit: |_| {
                comments.write().push(comment_field.get().clone());
                comment_field.set(String::new());
            },
            input {
                value: "{comment_field}",
                oninput: |event| comment_field.set(event.value.clone()),
            }
            input {
                r#type: "submit",
            }
        },
        comments_rendered,
    ))
    // ANCHOR_END: render_list
}
