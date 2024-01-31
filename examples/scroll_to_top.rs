use dioxus::prelude::*;

fn main() {
    launch_desktop(app);
}

fn app() -> Element {
    let mut header_element = use_signal(|| None);

    rsx! {
        div {
            h1 {
                onmounted: move |cx| header_element.set(Some(cx.inner().clone())),
                "Scroll to top example"
            }

            for i in 0..100 {
                div { "Item {i}" }
            }

            button {
                onclick: move |_| async move {
                    if let Some(header) = header_element.cloned() {
                        let _ = header.scroll_to(ScrollBehavior::Smooth).await;
                    }
                },
                "Scroll to top"
            }
        }
    }
}
