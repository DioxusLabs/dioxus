use std::collections::HashMap;

use dioxus::prelude::*;
fn main() {}

fn app(cx: Scope) -> Element {
    let val = use_ref(&cx, || HashMap::<u32, String>::new());

    // Pull the value out locally
    let p = val.read();

    // to get an &HashMap we have to "reborrow" through the RefCell
    // Be careful: passing this into children might cause a double borrow of the RefCell and a panic
    let g = &*p;

    cx.render(rsx! {
        div {
            "hi"
        }
    })
}
