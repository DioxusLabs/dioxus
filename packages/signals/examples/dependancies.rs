use dioxus::prelude::*;

fn main() {
    launch(app);
}

fn app() -> Element {
    let mut signal = use_signal(|| 0);

    use_future(move || async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            signal += 1;
        }
    });

    let mut local_state = use_signal(|| 0);

    let computed = use_memo_with_dependencies((&local_state(),), move |(local_state,)| {
        local_state * 2 + signal.cloned()
    });

    println!("Running app");

    rsx! {
        button { onclick: move |_| local_state.set(local_state() + 1), "Add one" }
        div { "{computed}" }
    }
}
