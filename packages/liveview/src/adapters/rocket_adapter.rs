use crate::LiveViewPool;
use crate::LiveviewRouter;
use crate::{LiveViewError, LiveViewSocket};
use rocket::futures::{SinkExt, StreamExt};
use rocket::response::content::RawHtml;
use rocket::{get, routes, State};
use rocket_ws::Channel;
use rocket_ws::WebSocket;
use rocket_ws::{result::Error, stream::DuplexStream, Message};
use std::sync::Arc;

/// Convert a Rocket WebSocket into a `LiveViewSocket`.
///
/// This is required to launch a LiveView app using the Rocket web framework.
pub fn rocket_socket(stream: DuplexStream) -> impl LiveViewSocket {
    stream
        .map(transform_rx)
        .with(transform_tx)
        .sink_map_err(|_| LiveViewError::SendingFailed)
}

fn transform_rx(message: Result<Message, Error>) -> Result<Vec<u8>, LiveViewError> {
    message
        .map_err(|_| LiveViewError::SendingFailed)?
        .into_text()
        .map(|s| s.into_bytes())
        .map_err(|_| LiveViewError::SendingFailed)
}

async fn transform_tx(message: Vec<u8>) -> Result<Message, Error> {
    Ok(Message::Binary(message))
}

impl LiveviewRouter for rocket::Rocket<rocket::Build> {
    fn create_default_liveview_router() -> Self {
        Self::build()
    }

    fn with_virtual_dom(
        self,
        route: &str,
        app: impl Fn() -> dioxus_core::prelude::VirtualDom + Send + Sync + 'static,
    ) -> Self {
        #[get("/")]
        async fn index(request: &rocket::route::Route) -> RawHtml<String> {
            let route = request.uri.base();

            let glue = crate::interpreter_glue(&format!("{route}/ws",));

            let title = crate::app_title();

            RawHtml(format!(
                r#"
        <!DOCTYPE html>
        <html>
            <head> <title>{title}</title>  </head>
            <body> <div id="main"></div> </body>
            {glue}
        </html>
        "#
            ))
        }

        #[get("/ws")]
        fn ws(ws: WebSocket, app: &State<LiveviewApp>) -> Channel<'static> {
            let app = app.inner();
            let pool = app.pool.clone();
            let app = app.app.clone();

            ws.channel(move |stream| {
                Box::pin(async move {
                    let _ = pool
                        .launch_virtualdom(crate::rocket_socket(stream), move || app())
                        .await;
                    Ok(())
                })
            })
        }

        struct LiveviewApp {
            app: Arc<dyn Fn() -> dioxus_core::prelude::VirtualDom + Send + Sync + 'static>,
            pool: LiveViewPool,
        }

        let app = Arc::new(app);

        let view = crate::LiveViewPool::new();

        self.manage(LiveviewApp {
            app: app,
            pool: view,
        })
        .mount(route, routes![index, ws])
    }

    async fn start(self, address: impl Into<std::net::SocketAddr>) {
        let address = address.into();

        let figment = self
            .figment()
            .clone()
            .merge((rocket::Config::PORT, address.port()))
            .merge((rocket::Config::ADDRESS, address.ip()));

        self.configure(figment)
            .ignite()
            .await
            .expect("Failed to ignite rocket")
            .launch()
            .await
            .expect("Failed to launch rocket");
    }
}
