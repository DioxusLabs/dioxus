use dioxus::prelude::*;
use std::time::Duration;

fn main() {
    dioxus_desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    let mut count = dioxus_signals::use_signal(cx, || 0);

    use_future!(cx, || async move {
        loop {
            count += 1;
            tokio::time::sleep(Duration::from_millis(400)).await;
        }
    });

    cx.render(rsx! {
        h1 { "High-Five counter: {count}" }
        button { onclick: move |_| count += 1, "Up high!" }
        button { onclick: move |_| count -= 1, "Down low!" }

        if count.value() > 5 {
            rsx!{ h2 { "High five!" } }
        }
    })
}
