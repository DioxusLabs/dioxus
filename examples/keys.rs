use std::cell::RefCell;

use crossterm::event::KeyEvent;
use dioxus::prelude::*;
use rink::InputHandler;

fn main() {
    rink::launch(app);
}

fn app(cx: Scope) -> Element {
    let (key, set_key) = use_state(&cx, || "".to_string());
    let (mouse, set_mouse) = use_state(&cx, || "".to_string());
    let (size, set_size) = use_state(&cx, || "".to_string());

    cx.render(rsx! {
        div {
            width: "100%",
            height: "10px",
            background_color: "red",
            justify_content: "center",
            align_items: "center",

            rink::InputHandler {
                onkeydown: move |evt: KeyEvent| {
                    set_key(format!("{evt:?}"));
                },
            },
            rink::InputHandler {
                onmousedown: move |evt| {
                    set_mouse(format!("{evt:?}"));
                },
            },
            rink::InputHandler {
                onresize: move |dims| {
                    set_size(format!("{dims:?}"));
                },
            },
            "keyboard: {key}
            mouse: {mouse}
            resize: {size}",
        }
    })
}

fn app2<'a>(cx: Scope<'a>) -> Element<'a> {
    let (count, set_count) = use_state(&cx, || 0);

    cx.render(rsx! {
        div {
            width: "100%",
            height: "10px",
            background_color: "red",
            justify_content: "center",
            align_items: "center",
            oninput: move |_| set_count(count + 1),
            "Hello world!",
            h1 {},
        }
    })
}
