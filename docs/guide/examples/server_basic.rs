#![allow(non_snake_case, unused)]
use dioxus::prelude::*;

#[tokio::main]
async fn main() {
    #[cfg(feature = "ssr")]
    {
        use dioxus_fullstack::prelude::*;

        let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 8080));
        axum::Server::bind(&addr)
            .serve(
                axum::Router::new()
                    .serve_dioxus_application("", ServeConfigBuilder::new(app, ()))
                    .into_make_service(),
            )
            .await
            .unwrap();
    }
}

fn app(cx: Scope) -> Element {
    let mut count = use_state(cx, || 0);

    cx.render(rsx! {
        h1 { "High-Five counter: {count}" }
        button { onclick: move |_| count += 1, "Up high!" }
        button { onclick: move |_| count -= 1, "Down low!" }
    })
}
