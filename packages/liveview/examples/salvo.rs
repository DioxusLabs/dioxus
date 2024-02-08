use dioxus::prelude::*;
use dioxus_liveview::LiveViewPool;
use salvo::affix;
use salvo::prelude::*;
use std::net::SocketAddr;
use std::sync::Arc;

fn app() -> Element {
    let mut num = use_signal(|| 0);

    rsx! {
        div {
            "hello salvo! {num}"
            button { onclick: move |_| num += 1, "Increment" }
        }
    }
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let addr = "127.0.0.1:3030";
    let acceptor = TcpListener::new(addr).bind().await;

    let view = LiveViewPool::new();

    let router = Router::new()
        .hoop(affix::inject(Arc::new(view)))
        .get(index)
        .push(Router::with_path("ws").get(connect));

    println!("Listening on http://{}", addr);

    Server::new(acceptor).serve(router).await;
}

#[handler]
fn index(res: &mut Response) {
    let addr: SocketAddr = ([127, 0, 0, 1], 3030).into();
    res.render(Text::Html(format!(
        r#"
            <!DOCTYPE html>
            <html>
                <head> <title>Dioxus LiveView with Salvo</title>  </head>
                <body> <div id="main"></div> </body>
                {glue}
            </html>
            "#,
        glue = dioxus_liveview::interpreter_glue(&format!("ws://{addr}/ws"))
    )));
}

#[handler]
async fn connect(
    req: &mut Request,
    depot: &mut Depot,
    res: &mut Response,
) -> Result<(), StatusError> {
    let view = depot.obtain::<Arc<LiveViewPool>>().unwrap().clone();

    WebSocketUpgrade::new()
        .upgrade(req, res, |ws| async move {
            _ = view.launch(dioxus_liveview::salvo_socket(ws), app).await;
        })
        .await
}
