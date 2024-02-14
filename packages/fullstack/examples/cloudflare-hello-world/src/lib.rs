use dioxus::prelude::*;

#[cfg(feature = "server")]
const INDEX_HTML: &str = include_str!("../dist/index.html");

#[cfg(feature = "server")]
#[worker::event(start)]
fn start() {
    use tracing_subscriber::prelude::*;
    use tracing_web::MakeWebConsoleWriter;

    console_error_panic_hook::set_once();

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_ansi(true)
        .without_time()
        .with_level(false)
        .with_writer(MakeWebConsoleWriter::new().with_pretty_level());
    tracing_subscriber::registry().with(fmt_layer).init();

    GetServerData::register_explicit().unwrap();
    PostServerData::register_explicit().unwrap()
}

#[cfg(feature = "server")]
#[worker::event(fetch)]
async fn fetch(
    req: worker::Request,
    env: worker::Env,
    ctx: worker::Context,
) -> worker::Result<worker::Response> {
    let virtual_dom_factory = move || {
        let vdom = VirtualDom::new(app);
        // for context in &contexts {
        //     vdom.insert_any_root_context(context());
        // }
        vdom
    };
    let cfg = dioxus::prelude::ServeConfig::builder()
        .index_html(INDEX_HTML.to_string())
        .incremental(IncrementalRendererConfig::new().clear_cache(false))
        .build();

    fetch_dioxus_application("", cfg, virtual_dom_factory, req, env, ctx).await
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

#[server]
async fn post_server_data(data: String) -> Result<(), ServerFnError> {
    tracing::info!("Server received: {}", data);

    Ok(())
}

#[server]
async fn get_server_data() -> Result<String, ServerFnError> {
    Ok(reqwest::get("https://httpbin.org/ip").await?.text().await?)
}
