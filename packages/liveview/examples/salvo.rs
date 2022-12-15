#[cfg(not(feature = "salvo"))]
fn main() {}

#[cfg(feature = "salvo")]
#[tokio::main]
async fn main() {
    use std::sync::Arc;

    use dioxus_core::{Element, LazyNodes, Scope};
    use dioxus_liveview as liveview;
    use dioxus_liveview::LiveView;
    use salvo::extra::affix;
    use salvo::extra::ws::WsHandler;
    use salvo::prelude::*;

    fn app(cx: Scope) -> Element {
        cx.render(LazyNodes::new(|f| f.text(format_args!("hello world!"))))
    }

    pretty_env_logger::init();

    let addr = ([127, 0, 0, 1], 3030);

    // todo: compactify this routing under one liveview::app method
    let view = liveview::new(addr);
    let router = Router::new()
        .hoop(affix::inject(Arc::new(view)))
        .get(index)
        .push(Router::with_path("app").get(connect));
    Server::new(TcpListener::bind(addr)).serve(router).await;

    #[handler]
    fn index(depot: &mut Depot, res: &mut Response) {
        let view = depot.obtain::<Arc<Liveview>>().unwrap();
        let body = view.body("<title>Dioxus LiveView</title>");
        res.render(Text::Html(body));
    }

    #[handler]
    async fn connect(
        req: &mut Request,
        depot: &mut Depot,
        res: &mut Response,
    ) -> Result<(), StatusError> {
        let view = depot.obtain::<Arc<Liveview>>().unwrap().clone();
        let fut = WsHandler::new().handle(req, res)?;
        let fut = async move {
            if let Some(ws) = fut.await {
                view.upgrade_salvo(ws, app).await;
            }
        };
        tokio::task::spawn(fut);
        Ok(())
    }
}
