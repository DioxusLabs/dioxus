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

    use_effect(move || {
        println!("High-Five counter: {}", count());
    });

    rsx! {
        div { "High-Five counter: {count}" }
    }
}
