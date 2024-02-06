use crate::LiveViewPool;
use crate::LiveviewRouter;
use crate::{LiveViewError, LiveViewSocket};
use futures_util::{SinkExt, StreamExt};
use salvo::conn::TcpListener;
use salvo::http::StatusError;
use salvo::websocket::WebSocketUpgrade;
use salvo::websocket::{Message, WebSocket};
use salvo::writing::Text;
use salvo::Listener;
use salvo::Server;
use salvo::{handler, Depot, Request, Response, Router};
use std::sync::Arc;

/// Convert a Salvo WebSocket into a `LiveViewSocket`.
///
/// This is required to launch a LiveView app using the Salvo web framework.
pub fn salvo_socket(ws: WebSocket) -> impl LiveViewSocket {
    ws.map(transform_rx)
        .with(transform_tx)
        .sink_map_err(|_| LiveViewError::SendingFailed)
}

fn transform_rx(message: Result<Message, salvo::Error>) -> Result<Vec<u8>, LiveViewError> {
    let as_bytes = message.map_err(|_| LiveViewError::SendingFailed)?;

    Ok(as_bytes.into())
}

async fn transform_tx(message: Vec<u8>) -> Result<Message, salvo::Error> {
    Ok(Message::binary(message))
}

#[derive(Clone)]
struct LiveviewApp {
    app: Arc<dyn Fn() -> dioxus_core::prelude::VirtualDom + Send + Sync + 'static>,
    pool: Arc<LiveViewPool>,
}

impl LiveviewRouter for Router {
    fn create_default_liveview_router() -> Self {
        Self::new()
    }

    fn with_virtual_dom(
        self,
        route: &str,
        app: impl Fn() -> dioxus_core::prelude::VirtualDom + Send + Sync + 'static,
    ) -> Self {
        let app = Arc::new(app);

        let view = crate::LiveViewPool::new();

        self.push(
            Router::with_path(route)
                .hoop(salvo::affix::inject(LiveviewApp {
                    app: app,
                    pool: Arc::new(view),
                }))
                .get(index)
                .push(Router::with_path("ws").get(connect)),
        )
    }

    async fn start(self, address: impl Into<std::net::SocketAddr>) {
        let address = address.into();

        let acceptor = TcpListener::new(address).bind().await;
        Server::new(acceptor).serve(self).await;
    }
}

#[handler]
fn index(req: &mut Request, res: &mut Response) {
    let base = req.uri().path();
    let title = crate::app_title();

    res.render(Text::Html(format!(
        r#"
            <!DOCTYPE html>
            <html>
                <head> <title>{title}</title>  </head>
                <body> <div id="main"></div> </body>
                {glue}
            </html>
            "#,
        glue = crate::interpreter_glue(&format!("{base}/ws"))
    )));
}

#[handler]
async fn connect(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> Result<(), StatusError> {
    let app = depot.obtain::<LiveviewApp>().unwrap().clone();
    let view = app.pool.clone();
    let app = app.app.clone();

    WebSocketUpgrade::new()
        .upgrade(req, res, |ws| async move {
            _ = view
                .launch_virtualdom(crate::salvo_socket(ws), move || app())
                .await;
        })
        .await
}
