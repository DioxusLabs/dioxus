pub fn main() {
    #[cfg(feature = "client")]
    wasm_bindgen_futures::spawn_local(async { client::app().await.unwrap() });

    #[cfg(feature = "server")]
    async_std::task::block_on(async { server::app().await.expect("") });
}

/// ===============================
///  Common code (shared types)
/// ===============================
#[derive(PartialEq, strum::EnumIter, strum::Display, strum::AsRefStr, strum::EnumString)]
pub enum SelectedStream {
    Football,
    Hockey,
    Socker,
}

/// Client-specific code
#[cfg(feature = "client")]
mod client {
    use super::*;
    use dioxus_core::prelude::*;
    use strum::IntoEnumIterator;

    pub async fn app() -> anyhow::Result<()> {
        Ok(dioxus_web::WebsysRenderer::start(APP).await)
    }

    static APP: FC<()> = |cx| {
        todo!()
        // let (selected_stream, set_stream) = use_state(cx, || SelectedStream::Football);

        // let opts = SelectedStream::iter().map(|name| rsx! { option { "{name}", value: "{name}" } });

        // cx.render(rsx! {
        //     div {
        //         h1 { "Tide basic CRUD app" }
        //         h2 { "Chosen stream: {selected_stream}" }
        //         select {
        //             value: {selected_stream.as_ref()}
        //             "{selected_stream}"
        //             {opts}
        //         }
        //     }
        // })
    };
}

/// Server-specific code
#[cfg(feature = "server")]
mod server {
    use async_std::sync::RwLock;
    pub use log::info;
    use std::sync::Arc;
    use tide::Request;
    use tide_websockets::{Message, WebSocket, WebSocketConnection};

    use crate::SelectedStream;

    // type ServerRequest = Request<Arc<RwLock<()>>>;
    type ServerRequest = Request<()>;
    // type ServerRequest = Request<Arc<RwLock<ServerState>>>;

    static CLIENT_PATH: &'static str = "";

    pub async fn app() -> anyhow::Result<()> {
        let mut app = tide::new();

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

    use dioxus_core::prelude::*;

    #[derive(PartialEq, Props)]
    struct SreamListProps {
        selected_stream: SelectedStream,
    }

    static STREAM_LIST: FC<SreamListProps> = |cx| {
        //
        let g = match cx.selected_stream {
            SelectedStream::Football => cx.render(rsx! {
                li {
                    "watch football!"
                }
            }),
            SelectedStream::Hockey => cx.render(rsx! {
                li {
                    "watch football!"
                }
            }),
            SelectedStream::Socker => cx.render(rsx! {
                li {
                    "watch football!"
                }
            }),
        };

        cx.render(rsx! {
            div {

            }
        })
    };
}
