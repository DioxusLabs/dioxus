use dioxus::prelude::*;

fn main() {
    launch_desktop(app);
}

fn app() -> Element {
    let mut state = use_signal(|| 0);

    use_future(move || async move {
        loop {
            state += 1;
            tokio::time::sleep(std::time::Duration::from_millis(1)).await;
        }
    });

    rsx! {
        button { onclick: move |_| state.set(0), "reset" }
        for _ in 0..10000 {
            div { "hello desktop! {state}" }
        }
    }
}
