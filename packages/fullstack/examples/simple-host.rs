use dioxus::prelude::*;
use dioxus_fullstack::{codec::Json, make_server_fn, AxumServerFn, Http, Websocket};
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
        return Ok(dioxus_fullstack::fetch::fetch("/thing")
            .method("POST")
            .json(&serde_json::json!({ "a": a, "b": b }))
            .send()
            .await?
            .json::<()>()
            .await?);
    }

    // if we do have the server feature, we can run the code directly
    #[cfg(feature = "server")]
    {
        async fn run_user_code(a: i32, b: String) -> dioxus::Result<()> {
            println!("Doing the thing on the server with {a} and {b}");
            Ok(())
        }

        inventory::submit! {
            AxumServerFn::new(
                http::Method::GET,
                "/thing",
                |req| {
                    Box::pin(async move {
                        todo!()
                    })
                },
                None
            )
        }

        return run_user_code(a, b).await;
    }

    #[allow(unreachable_code)]
    {
        unreachable!()
    }
}

async fn make_websocket() -> dioxus::Result<Websocket> {
    Ok(Websocket::new(|tx, rx| async move {
        //
    }))
}

make_server_fn!(
    #[get("/thing/:a/:b")]
    pub async fn do_thing2(a: i32, b: String) -> dioxus::Result<()> {
        Ok(())
    }
);
