use dioxus::prelude::*;

fn main() {
    dioxus::launch(app)
}

fn app() -> Element {
    let mut signal = use_signal(|| 0);
    let doubled = use_memo(move || signal * 2);

    rsx! {
        button {
            onclick: move |_| signal += 1,
            "Increase"
        }
        Child { signal: doubled }
    }
}

#[component]
fn Child(signal: ReadSignal<usize>) -> Element {
    rsx! { "{signal}" }
}
