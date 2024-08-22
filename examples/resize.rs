//! Run a callback
//!
//! Whenever an Element is finally mounted to the Dom, its data is available to be read.
//! These fields can typically only be read asynchronously, since various renderers need to release the main thread to
//! perform layout and painting.

use dioxus::prelude::*;
use dioxus_elements::geometry::euclid::Size2D;

fn main() {
    launch(app);
}

fn app() -> Element {
    let mut dimensions = use_signal(Size2D::zero);

    rsx!(
        document::Stylesheet { href: asset!("/examples/assets/read_size.css") }
        div {
            width: "50%",
            height: "50%",
            background_color: "red",
            onresize: move |evt| dimensions.set(evt.data().get_content_box_size().unwrap()),
            "This element is {dimensions():?}"
        }
    )
}
