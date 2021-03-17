pub fn main() {
    #[cfg(feature = "client")]
    wasm_bindgen_futures::spawn_local(async { client::app().await.unwrap() });

    #[cfg(feature = "server")]
    async_std::task::block_on(async { server::app().await.expect("") });
}

#[derive(strum::EnumIter, strum::Display, strum::AsRefStr)]
pub enum SelectedStream {
    Football,
    Hockey,
    Socker,
}

#[cfg(feature = "client")]
mod client {
    use super::*;
    use dioxus_core::prelude::*;
    use dioxus_web::WebsysRenderer;
    use strum::IntoEnumIterator;

    pub async fn app() -> anyhow::Result<()> {
        Ok(dioxus_web::WebsysRenderer::start(APP).await)
    }

    static APP: FC<()> = |ctx, props| {
        let (selected_stream, set_stream) = use_state(&ctx, || SelectedStream::Football);

        let options = SelectedStream::iter().map(|name| {
            rsx! { option { "{name}", value: "{name}" } }
        });

        ctx.render(rsx! {
            div {
                h1 { "Tide basic CRUD app" }
                h2 { "Chosen stream: {selected_stream}" }
                select {
                    value: {selected_stream.as_ref()}
                    "{selected_stream}"
                    {options}
                }
            }
        })
    };
}

#[cfg(feature = "server")]
mod server {
    use async_std::sync::RwLock;
    pub use log::info;
    use std::sync::Arc;
    use tide::Request;
    use tide_websockets::{Message, WebSocket, WebSocketConnection};

    // type ServerRequest = Request<Arc<RwLock<()>>>;
    type ServerRequest = Request<()>;
    // type ServerRequest = Request<Arc<RwLock<ServerState>>>;

    static CLIENT_PATH: &'static str = "";

    pub async fn app() -> anyhow::Result<()> {
        let mut app = tide::new();
        // let mut app = tide::with_state(Arc::new(RwLock::new(())));
        // let mut app = tide::with_state(ServerState::new());

        // for all examples:
        // assume the bundle exists at ../public

        app.at("")
            .serve_dir(format!("{}/pkg", CLIENT_PATH))
            .expect("Cannot serve directory");

        app.at("/updates").get(WebSocket::new(socket_handler));

        let addr = "0.0.0.0:9001";
        log::info!("Congrats! Server is up and running at http://{}", addr);
        app.listen(addr).await?;

        Ok(())
    }

    async fn socket_handler(
        request: ServerRequest,
        stream: WebSocketConnection,
    ) -> tide::Result<()> {
        // clone the receiver channel
        // subscribe to any updates
        // let receiver = request.state().read().await.receiver.clone();
        // while let Ok(evt) = receiver.recv().await {
        //     let response_msg = serde_json::to_string(&evt)?;
        //     stream.send_string(response_msg).await?;
        // }

        Ok(())
    }
}
