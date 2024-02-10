//! Run with:
//!
//! ```sh
//! dx serve --platform web --features web
//! ```

use dioxus::prelude::*;

#[cfg(all(feature = "web", not(feature = "server")))]
fn main() {
    tracing_wasm::set_as_global_default();
    dioxus_web::launch::launch_cfg(
        app,
        dioxus_web::Config::default().hydrate(false),
    );
}

fn app() -> Element {
    let mut count = use_signal(|| 0);
    let text = use_signal(|| "...".to_string());
    // let server_future = use_server_future(get_server_data)?;

    rsx! {
        h1 { "High-Five counter: {count}" }
        button { onclick: move |_| count += 1, "Up high!" }
        button { onclick: move |_| count -= 1, "Down low!" }
        // button {
        //     onclick: move |_| {
        //         to_owned![text];
        //         async move {
        //             if let Ok(data) = get_server_data().await {
        //                 println!("Client received: {}", data);
        //                 text.set(data.clone());
        //                 post_server_data(data).await.unwrap();
        //             }
        //         }
        //     },
        //     "Run a server function!"
        // }
        "Server said: {text}"
        // "{server_future.state():?}"
    }
}
