use std::prelude::rust_2024::Future;

use dioxus_fullstack::{codec::Json, AxumServerFn, Http};

#[tokio::main]
async fn main() {}

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
        use http::Method;

        async fn run_user_code(a: i32, b: String) -> dioxus::Result<()> {
            println!("Doing the thing on the server with {a} and {b}");
            Ok(())
        }

        inventory::submit! {
            AxumServerFn::new(
                Method::GET,
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
}

/*
dioxus::Result
-> rpc errors
-> render errors
-> request errors
-> dyn any?

or... just dyn any and then downcast?


ServerFn is a struct...
with encoding types...?
*/

// #[link_section = "server_fns"]
// static DO_THING_SERVER_FN: dioxus_fullstack::ServerFnObject =
//     register_run_on_server("/thing", |req: dioxus::Request| async move {
//         let a = req.path("a").and_then(|s| s.parse().ok()).unwrap_or(0)?;
//         let b = req.path("b").unwrap_or_default();
//         let result = run_user_code(do_thing, (a, b)).await;
//         dioxus::Response::new().json(&result)
//     });

// struct UserCodeServerFn;
// impl ServerFn for UserCodeServerFn {
//     const PATH: &'static str = "/thing";

//     type Output = Json;
//     type Protocol = Http<Self, Json>;

//     fn run_body(
//         self,
//     ) -> impl Future<Output = Result<Self::Output, dioxus_fullstack::HybridError>> + Send
//     {
//         todo!()
//     }
// }
