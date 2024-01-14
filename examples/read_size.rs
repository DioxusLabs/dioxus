#![allow(clippy::await_holding_refcell_ref)]
use std::rc::Rc;

use dioxus::{html::geometry::euclid::Rect, prelude::*};

fn main() {
    dioxus_desktop::launch_cfg(
        app,
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
    let div_element: Signal<Option<Rc<MountedData>>> = use_signal(|| None);

    let dimentions = use_signal(Rect::zero);

    rsx!(
        div {
            width: "50%",
            height: "50%",
            background_color: "red",
            onmounted: move |cx| {
                div_element.set(Some(cx.inner().clone()));
            },
            "This element is {dimentions.read():?}"
        }

        button {
            onclick: move |_| {
                to_owned![div_element, dimentions];
                async move {
                    let read = div_element.read();
                    let client_rect = read.as_ref().map(|el| el.get_client_rect());
                    if let Some(client_rect) = client_rect {
                        if let Ok(rect) = client_rect.await {
                            dimentions.set(rect);
                        }
                    }
                }
            },
            "Read dimentions"
        }
    )
}
