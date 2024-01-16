#![allow(clippy::await_holding_refcell_ref)]
use std::rc::Rc;

use dioxus::{html::geometry::euclid::Rect, prelude::*};

fn main() {
    LaunchBuilder::new(app).cfg(
        dioxus_desktop::Config::default().with_custom_head(
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
    );
}

fn app() -> Element {
    let mut div_element = use_signal(|| None as Option<Rc<MountedData>>);
    let mut dimensions = use_signal(Rect::zero);

    let mut read_dims = move |_| async move {
        let read = div_element.read();
        let client_rect = read.as_ref().map(|el| el.get_client_rect());
        if let Some(client_rect) = client_rect {
            if let Ok(rect) = client_rect.await {
                dimensions.set(rect);
            }
        }
    };

    render!(
        div {
            width: "50%",
            height: "50%",
            background_color: "red",
            onmounted: move |cx| div_element.set(Some(cx.inner().clone())),
            "This element is {dimensions():?}"
        }

        button { onclick: read_dims, "Read dimensions" }
    )
}
