use dioxus::prelude::*;
use dioxus_signals::*;

fn main() {
    tracing_subscriber::fmt::init();
    dioxus_desktop::launch(App);
}

#[component]
fn App(cx: Scope) -> Element {
    let mut signal = use_signal_sync(cx, || 0);
    cx.use_hook(|| {
        std::thread::spawn(move || loop {
            std::thread::sleep(std::time::Duration::from_secs(1));
            signal += 1;
        })
    });

    render! {
        button {
            onclick: move |_| signal += 1,
            "Increase"
        }
        "{signal}"
    }
}
