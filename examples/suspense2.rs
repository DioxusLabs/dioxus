use dioxus::prelude::*;
use dioxus_desktop::{Config, LogicalSize, WindowBuilder};

fn main() {
    dioxus_desktop::launch(|cx| cx.render(rsx! { async_app {} }));
}

async fn async_app(cx: Scope<'_>) -> Element {
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    cx.render(rsx! {
        div {
            "hi!"
        }
    })
}
