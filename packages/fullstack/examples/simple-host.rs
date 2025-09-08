#[tokio::main]
async fn main() {}

async fn do_thing(a: i32, b: String) -> dioxus::Result<()> {
    // On the server, we register the function
    #[cfg(feature = "server")]
    #[link_section = "server_fns"]
    static DO_THING_SERVER_FN: dioxus::ServerFnObject =
        register_run_on_server("/thing", |req: dioxus::Request| async move {
            let a = req.path("a").and_then(|s| s.parse().ok()).unwrap_or(0)?;
            let b = req.path("b").unwrap_or_default();
            let result = run_user_code(do_thing, (a, b)).await;
            dioxus::Response::new().json(&result)
        });

    // On the server, if this function is called, then we just run the user code directly
    #[cfg(feature = "server")]
    return run_user_code(a, b).await;

    // Otherwise, we always use the ServerFn's client to call the URL
    fetch("http://localhost:8080/thing")
        .method("POST")
        .json(&serde_json::json!({ "a": a, "b": b }))
        .send()
        .await?
        .json::<()>()
        .await
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
