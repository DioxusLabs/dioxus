use dioxus::prelude::*;
use dioxus_fullstack::{make_server_fn, ServerFnSugar, ServerFunction, Websocket};
use http::Method;
use serde::Serialize;
use std::future::Future;

#[tokio::main]
async fn main() {
    dioxus_fullstack::launch(|| {
        rsx! {
            "hello world"
        }
    })
}

async fn do_thing(a: i32, b: String) -> dioxus::Result<()> {
    // If no server feature, we always make a request to the server
    if cfg!(not(feature = "server")) {
        return Ok(dioxus_fullstack::fetch::fetch(Method::POST, "/thing")
            .json(&serde_json::json!({ "a": a, "b": b }))
            .send()
            .await?
            .json::<()>()
            .await?);
    }

    // if we do have the server feature, we can run the code directly
    #[cfg(feature = "server")]
    {
        use dioxus_fullstack::{codec::GetUrl, HybridRequest};

        async fn run_user_code(a: i32, b: String) -> dioxus::Result<()> {
            println!("Doing the thing on the server with {a} and {b}");
            Ok(())
        }

        inventory::submit! {
            ServerFunction::new(
                http::Method::GET,
                "/thing",
                || {
                    todo!()
                },
            )
        }

        return run_user_code(a, b).await;
    }

    #[allow(unreachable_code)]
    {
        unreachable!()
    }
}

#[post("/thing", ws: axum::extract::WebSocketUpgrade)]
async fn make_websocket() -> dioxus::Result<Websocket<String, String>> {
    use axum::extract::ws::WebSocket;

    ws.on_upgrade(|mut socket| async move {
        while let Some(msg) = socket.recv().await {
            socket
                .send(axum::extract::ws::Message::Text("pong".into()))
                .await
                .unwrap();
        }
    });

    // Ok(WebSocket::new(|tx, rx| async move {
    //     //
    // }))
    todo!()
}

/*
parse out URL params
rest need to implement axum's FromRequest / extract
body: String
body: Bytes
payload: T where T: Deserialize (auto to Json, can wrap in other codecs)
extra items get merged as body, unless theyre also extractors?
hoist up FromRequest objects if they're just bounds
no State<T> extractors, use ServerState instead?

if there's a single trailing item, it's used as the body?

or, an entirely custom system, maybe based on names?
or, hoist up FromRequest objects into the signature?
*/

#[get("/thing/{a}/{b}?amount&offset")]
pub async fn do_thing23(
    a: i32,
    b: String,
    amount: Option<u32>,
    offset: Option<u32>,
    #[cfg(feature = "server")] headers: http::HeaderMap,
    #[cfg(feature = "server")] body: axum::body::Bytes,
) -> dioxus::Result<()> {
    Ok(())
}

fn register_some_serverfn() {
    // let r = axum::routing::get(handler);
    // let r = axum::routing::get_service(handler);
}
