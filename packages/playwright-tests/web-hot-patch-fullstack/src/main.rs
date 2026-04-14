use dioxus::prelude::*;

const CSS: Asset = asset!("/assets/style.css");
const IMAGE: Asset = asset!("/assets/toasts.png");

fn app() -> Element {
    let mut num = use_signal(|| 0);

    rsx! {
        document::Link {
            href: CSS,
            rel: "stylesheet",
        }
        img {
            id: "toasts",
            src: IMAGE,
        }
        button {
            id: "increment-button",
            onclick: move |_| async move {
                let increment_amount = get_count().await.unwrap();
                *num.write() += increment_amount;
            },
            "Click me! Count: {num}"
        }
    }
}

#[post("/api/get_count")]
async fn get_count() -> Result<i32> {
    Ok(1)
}

fn main() {
    dioxus::launch(app);
}
