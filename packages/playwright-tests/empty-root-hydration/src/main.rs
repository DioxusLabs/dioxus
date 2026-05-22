use dioxus::prelude::*;

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    let mut text = use_signal(String::new);
    let mut show = use_signal(|| false);

    use_effect(move || {
        text.set("root text ready".to_string());
        show.set(true);
    });

    rsx! {
        "{text}"
        if show() {
            div { id: "late-empty-root", "late root ready" }
        }
    }
}
