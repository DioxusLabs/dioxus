use dioxus::prelude::*;

fn main() {
    launch_desktop(app);
}

fn app() -> Element {
    let mut state = use_signal(|| 0);
    let mut depth = use_signal(|| 1 as usize);

    if depth() == 5 {
        return rsx! {
            div { "Max depth reached" }
            button { onclick: move |_| depth -= 1, "Remove depth" }
        };
    }

    let mut items = use_memo(move || (0..depth()).map(|f| f as _).collect::<Vec<isize>>());

    rsx! {
        button { onclick: move |_| state += 1, "Increment" }
        button { onclick: move |_| depth += 1, "Add depth" }
        button {
            onclick: move |_| async move {
                depth += 1;
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                dbg!(items.read());
                // if depth() is 5, this will be the old since the memo hasn't been re-computed
                // use_memos are only re-computed when the signals they capture change
                // *and* they are used in the current render
                // If the use_memo isn't used, it can't be re-computed!
            },
            "Add depth with sleep"
        }
    }
}
