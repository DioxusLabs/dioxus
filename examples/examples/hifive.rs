//! An example where the dioxus vdom is running in a native thread, interacting with webview
//! Content is passed from the native thread into the webview
use dioxus_core as dioxus;
use dioxus_core::prelude::*;
fn main() {
    dioxus_desktop::launch(
        |builder| {
            builder
                .title("Test Dioxus App")
                .size(320, 480)
                .resizable(false)
                .debug(true)
        },
        (),
        App,
    )
    .expect("Webview finished");
}

static App: FC<()> = |cx| {
    let hifives = use_model(&cx, || 0);
    cx.render(rsx! {
        div {
            h1 { "Hi-fives collected: {hifives}" }
            button { "Hi five me!", onclick: move |_| *hifives.get_mut() += 1 }
        }
    })
};
