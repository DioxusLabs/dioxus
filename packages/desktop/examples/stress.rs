use dioxus::prelude::*;

fn app() -> Element {
    let state = use_signal(|| 0);
    use_future(|_| {
        to_owned![state];
        async move {
            loop {
                state += 1;
                tokio::time::sleep(std::time::Duration::from_millis(1)).await;
            }
        }
    });

    rsx! {
        button {
            onclick: move |_| {
                state.set(0);
            },
            "reset"
        }
        for _ in 0..10000 {
            div {
                "hello desktop! {state}"
            }
        }
    }
}

fn main() {
    dioxus_desktop::launch(app);
}
