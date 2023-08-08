use dioxus::prelude::*;
use dioxus_signals::use_signal;

fn main() {
    dioxus_desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    let mut count = use_signal(cx, || 0);

    use_future!(cx, || async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            count += 1;
            println!("current: {count}");
        }
    });

    cx.render(rsx! {
        div { "High-Five counter: {count}" }
    })
}
