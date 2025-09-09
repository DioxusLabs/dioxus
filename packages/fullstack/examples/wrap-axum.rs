use std::sync::Arc;

use axum::Router;
use bytes::Bytes;
use http::StatusCode;

#[tokio::main]
async fn main() {
    // Create the app
    let mut app: Router<Arc<DioxusAppState>> = Router::new();

    for sf in inventory::iter::<NewServerFunction> {
        app = app.route(sf.path, (sf.make_routing)());
    }

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    println!("listening on http://127.0.0.1:3000");
    axum::serve(listener, app.with_state(Arc::new(DioxusAppState {})))
        .await
        .unwrap();
}

#[derive(serde::Deserialize)]
struct QueryParams {
    a: i32,
    b: String,
    amount: Option<u32>,
    offset: Option<u32>,
}

#[derive(serde::Deserialize)]
struct BodyData {
    // bytes: Bytes,
}

struct DioxusAppState {}

// #[get("/thing/{a}/{b}?amount&offset")]
#[axum::debug_handler]
async fn do_thing23(
    state: axum::extract::State<Arc<DioxusAppState>>,
    params: axum::extract::Query<QueryParams>,
    #[cfg(feature = "server")] headers: http::HeaderMap,
    // #[cfg(feature = "server")] body: axum::extract::Json<BodyData>,
    // #[cfg(feature = "server")] body: axum::body::Bytes,
) -> Result<String, StatusCode> {
    Ok(format!(
        "a={} b={} amount={:?} offset={:?} headers={:#?}",
        params.a, params.b, params.amount, params.offset, headers
    ))
}

inventory::collect!(NewServerFunction);
inventory::submit!(NewServerFunction {
    path: "/thing/{a}/{b}/",
    method: http::Method::GET,
    make_routing: || axum::routing::get(do_thing23),
});
inventory::submit!(NewServerFunction {
    path: "/home",
    method: http::Method::GET,
    make_routing: || axum::routing::get(|| async { "hello world" }),
});

#[derive(Clone)]
struct NewServerFunction {
    make_routing: fn() -> axum::routing::MethodRouter<Arc<DioxusAppState>>,
    method: http::Method,
    path: &'static str,
}

fn make_routing() -> axum::routing::MethodRouter<Arc<DioxusAppState>> {
    axum::routing::get(do_thing23)
}

// fn it_works() {
//     make_the_thing(axum::routing::get(do_thing23));
// }

// // #[get("/thing/{a}/{b}?amount&offset")]
// #[axum::debug_handler]
// pub async fn do_thing23(
//     a: i32,
//     b: String,
//     amount: Option<u32>,
//     offset: Option<u32>,
//     #[cfg(feature = "server")] headers: http::HeaderMap,
//     #[cfg(feature = "server")] body: axum::body::Bytes,
// ) -> Result<String, StatusCode> {
//     Ok("".to_string())
// }

// fn make_the_thing(r: axum::routing::MethodRouter<Arc<DioxusAppState>>) {}

// // static CAN_YOU_BE_STATIC: NewServerFunction = NewServerFunction {
// //     path: "/thing/{a}/?amount&offset",
// //     // path: "/thing/{a}/{b}/?amount&offset",
// //     method: http::Method::GET,
// //     make_routing: || axum::routing::get(do_thing23),
// // };
