//! Nested Listeners
//!
//! This example showcases how to control event bubbling from child to parents.
//!
//! Both web and desktop support bubbling and bubble cancelation.

use dioxus::prelude::*;

fn main() {
    launch(app);
}

fn app() -> Element {
    rsx! {
        div {
            onclick: move |_| println!("clicked! top"),
            "- div"
            button {
                onclick: move |_| println!("clicked! bottom propagate"),
                "Propagate"
            }
            button {
                onclick: move |evt| {
                    println!("clicked! bottom no bubbling");
                    evt.stop_propagation();
                },
                "Dont propagate"
            }
            button {
                "Does not handle clicks - only propagate"
            }
        }
    }
}
