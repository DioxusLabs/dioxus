#![allow(non_snake_case, unused)]
use dioxus::prelude::*;
use dioxus_fullstack::prelude::*;

fn main() {
    #[cfg(feature = "web")]
    dioxus_web::launch_with_props(
        app,
        // Get the root props from the document
        get_root_props_from_document().unwrap_or_default(),
        dioxus_web::Config::new().hydrate(true),
    );
    #[cfg(feature = "ssr")]
    {
        use axum::extract::Path;
        use axum::extract::State;
        use axum::routing::get;
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async move {
                let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 8080));
                axum::Server::bind(&addr)
                    .serve(
                        axum::Router::new()
                            // Serve the dist folder with the static javascript and WASM files created by the dixous CLI
                            .serve_static_assets("./dist")
                            // Register server functions
                            .register_server_fns("")
                            // Connect to the hot reload server in debug mode
                            .connect_hot_reload()
                            // Render the application. This will serialize the root props (the intial count) into the HTML
                            .route(
                                "/",
                                get(move | State(ssr_state): State<SSRState>| async move { axum::body::Full::from(
                                    ssr_state.render(
                                        &ServeConfigBuilder::new(
                                            app,
                                            0,
                                        )
                                        .build(),
                                    )
                                )}),
                            )
                            // Render the application with a different intial count
                            .route(
                                "/:initial_count",
                                get(move |Path(intial_count): Path<usize>, State(ssr_state): State<SSRState>| async move { axum::body::Full::from(
                                    ssr_state.render(
                                        &ServeConfigBuilder::new(
                                            app,
                                            intial_count,
                                        )
                                        .build(),
                                    )
                                )}),
                            )
                            .with_state(SSRState::default())
                            .into_make_service(),
                    )
                    .await
                    .unwrap();
            });
    }
}

fn app(cx: Scope<usize>) -> Element {
    let mut count = use_state(cx, || *cx.props);

    cx.render(rsx! {
        h1 { "High-Five counter: {count}" }
        button { onclick: move |_| count += 1, "Up high!" }
        button { onclick: move |_| count -= 1, "Down low!" }
    })
}
