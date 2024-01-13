use dioxus::prelude::*;

fn main() {
    dioxus_desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    let header_element = use_ref(cx, || None);

    cx.render(rsx!(
        div {
            h1 {
                onmounted: move |cx| {
                    header_element.set(Some(cx.inner().clone()));
                },
                "Scroll to top example"
            }

            for i in 0..100 {
                div { "Item {i}" }
            }

            button {
                onclick: move |_| {
                    if let Some(header) = header_element.read().as_ref().cloned() {
                        cx.spawn(async move {
                            let _ = header.scroll_to(ScrollBehavior::Smooth).await;
                        });
                    }
                },
                "Scroll to top"
            }
        }
    ))
}
