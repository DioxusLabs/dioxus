use std::rc::Rc;

use dioxus::prelude::*;

fn main() {
    dioxus_desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    let login = use_callback!(cx, || |evt| async {
        //
    });

    cx.render(rsx! {
        button {
            onclick: login,
            "Click me!"
        }
    })
}
