use axum::{handler::Handler, Json};
use dioxus_fullstack::DioxusServerState;
use http::HeaderMap;

fn main() {}

fn assert_handler<T, F: Handler<T, DioxusServerState>>(_: F) -> T {
    todo!()
}

async fn handler1() -> &'static str {
    "Hello, World!"
}

async fn handler2(t: HeaderMap, body: Json<String>) -> &'static str {
    "Hello, World!"
}
async fn handler3(t: HeaderMap) -> &'static str {
    "Hello, World!"
}
async fn handler4(t: HeaderMap, t2: HeaderMap, t3: String) -> &'static str {
    "Hello, World!"
}

fn it_works() {
    let r = assert_handler(handler1);
    let r2 = assert_handler(handler2);
    let r3 = assert_handler(handler3);
    let r3 = assert_handler(handler4);
}

type H4 = (HeaderMap, HeaderMap, Json<String>);
