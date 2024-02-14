//! Read the size of elements using the MountedData struct.
//!
//! Whenever an Element is finally mounted to the Dom, its data is avaiable to be read.
//! These fields can typically only be read asynchronously, since various renderers need to release the main thread to
//! perform layout and painting.

use std::rc::Rc;

use dioxus::{html::geometry::euclid::Rect, prelude::*};

fn main() {
    LaunchBuilder::desktop()
        .with_cfg(
            dioxus::desktop::Config::default().with_custom_head(
                r#"
<style type="text/css">
    html, body {
        height: 100%;
        width: 100%;
        margin: 0;
    }
    #main {
        height: 100%;
        width: 100%;
    }
</style>
"#
                .to_owned(),
            ),
        )
        .launch(app);
}

fn app() -> Element {
    let mut div_element = use_signal(|| None as Option<Rc<MountedData>>);
    let mut dimensions = use_signal(Rect::zero);

    let read_dims = move |_| async move {
        let read = div_element.read();
        let client_rect = read.as_ref().map(|el| el.get_client_rect());

        if let Some(client_rect) = client_rect {
            if let Ok(rect) = client_rect.await {
                dimensions.set(rect);
            }
        }
    };

    rsx!(
        div {
            width: "50%",
            height: "50%",
            background_color: "red",
            onmounted: move |cx| div_element.set(Some(cx.data())),
            "This element is {dimensions():?}"
        }

        button { onclick: read_dims, "Read dimensions" }
    )
}
