use std::cell::RefCell;

use crossterm::event::KeyEvent;
use dioxus::prelude::*;
use rink::InputHandler;

fn main() {
    rink::launch(app);
}

fn app(cx: Scope) -> Element {
    let count = use_state(&cx, || 0);

    cx.render(rsx! {
        div {
            width: "100%",
            height: "10px",
            background_color: "red",
            justify_content: "center",
            align_items: "center",
            "Hello world!",

            // todo: enabling this will panic
            // rink::InputHandler {
            //     onkeydown: move |evt: KeyEvent| {
            //         use crossterm::event::KeyCode::*;
            //         match evt.code {
            //             Left => count += 1,
            //             Right => count -= 1,
            //             Up => count += 10,
            //             Down => count -= 10,
            //             _ => {},
            //         }
            //     },
            //     onmousedown: move |evt| {},
            //     onresize: move |dims| {
            //         println!("{:?}", dims);
            //     },
            // }
        }
    })
}

fn app2<'a>(cx: Scope<'a>) -> Element<'a> {
    let mut count = use_state(&cx, || 0);

    cx.render(rsx! {
        div {
            width: "100%",
            height: "10px",
            background_color: "red",
            justify_content: "center",
            align_items: "center",
            oninput: move |_| count += 1,
            "Hello world!",
            h1 {},
        }
    })
}
