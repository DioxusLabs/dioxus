use std::cell::RefCell;

use crossterm::event::{KeyCode, KeyEvent, MouseEvent};
use dioxus::prelude::*;
use rink::InputHandler;

fn main() {
    rink::launch(app);
}

fn app(cx: Scope) -> Element {
    let (key, set_key) = use_state(&cx, || KeyCode::Null);
    let (mouse, set_mouse) = use_state(&cx, || (0, 0));
    let (size, set_size) = use_state(&cx, || (0, 0));
    let (count, set_count) = use_state(&cx, || 0);

    cx.render(rsx! {
        div {
            width: "100%",
            height: "10px",
            background_color: "red",
            justify_content: "center",
            align_items: "center",
            flex_direction: "column",

            rink::InputHandler {
                onkeydown: move |evt: KeyEvent| {
                    use crossterm::event::KeyCode::*;
                    match evt.code {
                        Left => set_count(count + 1),
                        Right => set_count(count - 1),
                        Up => set_count(count + 10),
                        Down => set_count(count - 10),
                        _ => {},
                    }
                    set_key(evt.code);
                },
                onmousedown: move |evt: MouseEvent| {
                    set_mouse((evt.row, evt.column));
                },
                onresize: move |dims| {
                    set_size(dims);
                },
            },
            "count: {count:?}",
            "key: {key:?}",
            "mouse: {mouse:?}",
            "resize: {size:?}",
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
