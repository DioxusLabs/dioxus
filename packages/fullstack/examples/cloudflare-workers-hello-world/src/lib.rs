use dioxus::prelude::*;
use tracing_subscriber::prelude::*;
use tracing_web::MakeConsoleWriter;

#[cfg(feature = "server")]
#[worker::event(start)]
fn start() {
    console_error_panic_hook::set_once();

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_ansi(true)
        .without_time()
        .with_writer(MakeConsoleWriter);
    tracing_subscriber::registry().with(fmt_layer).init();

    GetServerData::register_explicit().unwrap();
    PostServerData::register_explicit().unwrap()
}

#[cfg(feature = "server")]
#[worker::event(fetch)]
async fn main(
    req: worker::Request,
    env: worker::Env,
    _ctx: worker::Context,
) -> worker::Result<worker::Response> {
    handle_dioxus_application("/api/", req, env).await
}

pub fn app() -> Element {
    let mut count = use_signal(|| 0);
    let text = use_signal(|| "...".to_string());
    // let server_future = use_server_future(get_server_data)?;

    rsx! {
        h1 { "High-Five counter: {count}" }
        button { onclick: move |_| count += 1, "Up high!" }
        button { onclick: move |_| count -= 1, "Down low!" }
        button {
            onclick: move |_| {
                to_owned![text];
                async move {
                    if let Ok(data) = get_server_data().await {
                        tracing::info!("Client received: {}", data);
                        text.set(data.clone());
                        post_server_data(data).await.unwrap();
                    }
                }
            },
            "Run a server function!"
        }
        "Server said: {text}"
        // "{server_future.state():?}"
    }
}

#[server("/api")]
async fn post_server_data(data: String) -> Result<(), ServerFnError> {
    tracing::info!("Server received: {}", data);

    Ok(())
}

#[server("/api")]
async fn get_server_data() -> Result<String, ServerFnError> {
    Ok(reqwest::get("https://httpbin.org/ip").await?.text().await?)
}
