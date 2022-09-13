#![allow(non_snake_case)]

use dioxus::prelude::*;

fn main() {
    dioxus_desktop::launch(App);
}

#[derive(PartialEq, Clone)]
struct Comment {
    content: String,
    id: usize,
}

pub fn App(cx: Scope) -> Element {
    // ANCHOR: render_list
    let comment_field = use_state(&cx, String::new);
    let mut next_id = use_state(&cx, || 0);
    let comments = use_ref(&cx, Vec::<Comment>::new);

    let comments_lock = comments.read();
    let comments_rendered = comments_lock.iter().map(|comment| {
        cx.render(rsx!(CommentComponent {
            key: "{comment.id}",
            comment: comment.clone(),
        }))
    });

    cx.render(rsx!(
        form {
            onsubmit: move |_| {
                comments.write().push(Comment {
                    content: comment_field.get().clone(),
                    id: *next_id.get(),
                });
                next_id += 1;

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

#[inline_props]
fn CommentComponent(cx: Scope, comment: Comment) -> Element {
    cx.render(rsx!(div {
        "Comment by anon:",
        p { "{comment.content}" }
        button { "Reply" },
    }))
}
