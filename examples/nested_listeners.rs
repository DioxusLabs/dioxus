//! Nested Listeners
//!
//! This example showcases how to control event bubbling from child to parents.
//!
//! Both web and desktop support bubbling and bubble cancelation.

use dioxus::prelude::*;

fn main() {
    dioxus::desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    cx.render(rsx! {
        div {
            onclick: move |_| println!("clicked! top"),
            button {
                onclick: move |_| println!("clicked! bottom propoate"),
                "Propogate"
            }
            button {
                onclick: move |evt| {
                    println!("clicked! bottom no bubbling");
                    evt.cancel_bubble();
                },
                "Dont propogate"
            }
            button {
                "Does not handle clicks"
            }
        }
    })
}
