#![cfg(not(target_family = "wasm"))]

use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use dioxus::prelude::*;
use dioxus_router::{
    history::{ControlledHistory, HistoryController, HistoryProvider, MemoryHistory},
    prelude::*,
};
use hyper::{
    header::{LOCATION, REFERRER_POLICY},
    service::{make_service_fn, service_fn},
    Body, Error, Response, Server, StatusCode,
};
use log::{error, info};

#[tokio::main]
async fn main() {
    env_logger::init();

    let make_svc = make_service_fn(|_| async {
        Ok::<_, Error>(service_fn(|request| async move {
            // prepare a controlled history
            let (mut controller, history) =
                HistoryController::new(Box::new(MemoryHistory::default()));

            // set request path
            let path = request
                .uri()
                .path_and_query()
                .map(|pq| pq.to_string())
                .unwrap_or_else(|| request.uri().path().to_string());
            info!("request to: {path}");
            controller.replace(path);

            // create and build the vdom
            let mut vdom = VirtualDom::new_with_props(App, AppProps { history });
            vdom.rebuild();

            let response = match controller.has_redirected() {
                // if the router hasn't redirected render the vdom and send it to the client
                false => Response::new(Body::from(format!(
                    r#"<!DOCTYPE html>
<html>
    <head>
        <title>Test Page</title>
    </head>
    <body>
        <div id="main">
            {app}
        </div>
    </body>
</html>"#,
                    app = dioxus_ssr::render_vdom(&vdom)
                ))),

                // if the router has redirected, send a 307 with the new location
                true => {
                    let url = controller
                        .get_external()
                        .unwrap_or(controller.current_path());

                    Response::builder()
                        .status(StatusCode::TEMPORARY_REDIRECT)
                        .header(LOCATION, &url)
                        .header(REFERRER_POLICY, "no-referrer")
                        .body(Body::from(format!(r#"<a href="{url}">redirect</a>"#,)))
                        .unwrap()
                }
            };

            Ok::<_, Error>(response)
        }))
    });

    // listen on 127.0.0.1:8000
    let address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8000);
    let server = Server::bind(&address).serve(make_svc);
    info!("server started, listening on {address}");

    if let Err(e) = server.await {
        error!("server error: {e}");
    }
}

#[derive(Props)]
struct AppProps {
    history: ControlledHistory,
}

impl PartialEq for AppProps {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

#[allow(non_snake_case)]
fn App(cx: Scope<AppProps>) -> Element {
    let routes = use_segment(&cx, || {
        Segment::new()
            .index(RcComponent(Index))
            .fixed("test", Route::new(RcComponent(Test)).name("test"))
            .fixed(
                "dioxus",
                Route::new("https://dioxuslabs.com").name("dioxus"),
            )
            .fallback(NamedTarget("", vec![], None))
    });

    let history = cx.use_hook(|| {
        let history = cx.props.history.clone();

        return move || -> Box<dyn HistoryProvider> { Box::new(history.clone()) };
    });

    cx.render(rsx! {
        Router {
            history: history,
            init_only: true,
            routes: routes.clone(),

            Outlet {}
        }
    })
}

#[allow(non_snake_case)]
fn Index(cx: Scope) -> Element {
    cx.render(rsx! {
        h1 { "Welcome to the SSR test!" }
        Link {
            target: NamedTarget("test", vec![], None),
            "Go to test page"
        }
        Link {
            target: NamedTarget("dioxus", vec![], None),
            "Go to dioxus"
        }
    })
}

#[allow(non_snake_case)]
fn Test(cx: Scope) -> Element {
    cx.render(rsx! {
        h2 { "This is the test page." }
        Link {
            target: NamedTarget("", vec![], None),
            "Return to home page"
        }
    })
}
