#![allow(clippy::await_holding_refcell_ref)]
use std::rc::Rc;

use dioxus::{html::geometry::euclid::Rect, prelude::*};
use dioxus_signals::use_signal;

fn main() {
    const CUSTOM_HEAD: &str = r#"
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
"#;

    dioxus_desktop::launch_cfg(
        app,
        dioxus_desktop::Config::default().with_custom_head(CUSTOM_HEAD.to_owned()),
    );
}

fn app(cx: Scope) -> Element {
    let element = use_signal(cx, || None as Option<Rc<MountedData>>);
    let dimensions = use_signal(cx, || Rect::zero());

    let read_dimensions = move |_| async move {
        let client_rect: Rect<f64, f64> = element().as_ref().unwrap().get_client_rect().await;

        dimensions.set(client_rect);
    };

    cx.render(rsx!(
        div {
            width: "50%",
            height: "50%",
            background_color: "red",
            onmounted: move |event| {
                println!("Mounted.....");
                element.set(Some(event.inner().clone()))
            },
            "This element is {dimensions:?}"
        }

        button {
            onclick: read_dimensions,
            "Read dimentions"
        }
    ))
}
