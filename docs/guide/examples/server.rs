mod basic {
    // ANCHOR: basic
    #![allow(non_snake_case)]
    use dioxus::prelude::*;
    use dioxus_server::prelude::*;

    #[tokio::main]
    async fn main() {
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

    fn app(cx: Scope) -> Element {
        let mut count = use_state(cx, || 0);

        cx.render(rsx! {
            h1 { "High-Five counter: {count}" }
            button { onclick: move |_| count += 1, "Up high!" }
            button { onclick: move |_| count -= 1, "Down low!" }
        })
    }
    // ANCHOR_END: basic
}

mod hydration {
    // ANCHOR: hydration
    #![allow(non_snake_case)]
    use dioxus::prelude::*;
    use dioxus_server::prelude::*;

    fn main() {
        #[cfg(feature = "web")]
        dioxus_web::launch_cfg(app, dioxus_web::Config::new().hydrate(true));
        #[cfg(feature = "ssr")]
        {
            tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(async move {
                    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 8080));
                    axum::Server::bind(&addr)
                        .serve(
                            axum::Router::new()
                                .serve_dioxus_application("", ServeConfigBuilder::new(app, ()))
                                .into_make_service(),
                        )
                        .await
                        .unwrap();
                });
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
    // ANCHOR_END: hydration
}

mod server_function {
    // ANCHOR: server_function
    #![allow(non_snake_case)]
    use dioxus::prelude::*;
    use dioxus_server::prelude::*;

    fn main() {
        #[cfg(feature = "web")]
        dioxus_web::launch_cfg(app, dioxus_web::Config::new().hydrate(true));
        #[cfg(feature = "ssr")]
        {
            // Register the server function before starting the server
            DoubleServer::register().unwrap();
            tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(async move {
                    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 8080));
                    axum::Server::bind(&addr)
                        .serve(
                            axum::Router::new()
                                // Serve Dioxus application automatically recognizes server functions and adds them to the API
                                .serve_dioxus_application("", ServeConfigBuilder::new(app, ()))
                                .into_make_service(),
                        )
                        .await
                        .unwrap();
                });
        }
    }

    fn app(cx: Scope) -> Element {
        let mut count = use_state(cx, || 0);

        cx.render(rsx! {
            h1 { "High-Five counter: {count}" }
            button { onclick: move |_| count += 1, "Up high!" }
            button { onclick: move |_| count -= 1, "Down low!" }
            button {
                onclick: move |_| {
                    to_owned![count];
                    async move {
                        // Call the server function just like a local async function
                        if let Ok(new_count) = double_server(*count.current()).await {
                            count.set(new_count);
                        }
                    }
                },
                "Double"
            }
        })
    }

    #[server(DoubleServer)]
    async fn double_server(number: u32) -> Result<u32, ServerFnError> {
        // Perform some expensive computation or access a database on the server
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        let result = number * 2;
        println!("server calculated {result}");
        Ok(result)
    }
    // ANCHOR_END: server_function
}
