use dioxus::prelude::*;
use dioxus_signals::use_signal;

fn main() {
    launch_desktop(app);
}

fn app() -> Element {
    let mut count = use_signal(|| 0);

    use_future(|| async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            count += 1;
            println!("current: {count}");
        }
    });

    render! {
        div { "High-Five counter: {count}" }
    }
}
