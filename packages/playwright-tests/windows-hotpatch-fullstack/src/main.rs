use dioxus::prelude::*;

const CSS: Asset = asset!("/assets/style.css");

fn app() -> Element {
    let mut num = use_signal(|| 0);

    rsx! {
        document::Link {
            href: CSS,
            rel: "stylesheet",
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
    let builder = dioxus::LaunchBuilder::new();

    #[cfg(feature = "desktop")]
    let builder = builder.with_cfg(
        dioxus::desktop::Config::new().with_windows_browser_args("--remote-debugging-port=8788"),
    );

    builder.launch(app);
}
