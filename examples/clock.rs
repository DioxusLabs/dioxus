use dioxus::prelude::*;

fn main() {
    launch_desktop(app);
}

fn app() -> Element {
    let mut count = use_signal(|| 0);

    use_future(|| async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            count += 1;
        }
    });

    rsx! {
        div { "High-Five counter: {count}" }
    }
}
