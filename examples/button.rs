use dioxus::prelude::*;

fn main() {
    dioxus_desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    cx.render(rsx! {
        button {
            onclick: |e| async move {
                println!("hello, desktop!");
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                println!("goodbye, desktop!");
            },
            "hello, desktop!"
        }
    })
}
