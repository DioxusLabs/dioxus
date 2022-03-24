#![allow(non_snake_case)]

//! This example proves that instantly resolving futures don't cause issues

use dioxus::prelude::*;

fn main() {
    dioxus::desktop::launch(App);
}

fn App(cx: Scope) -> Element {
    cx.render(rsx!(Demo {}))
}

fn Demo(cx: Scope) -> Element {
    let fut1 = use_future(&cx, (), |_| async move {
        std::thread::sleep(std::time::Duration::from_millis(100));
        10
    });

    cx.render(match fut1.value() {
        Some(value) => {
            let content = format!("content : {:?}", value);
            rsx!(div{ "{content}" })
        }
        None => rsx!(div{"computing!"}),
    })
}
