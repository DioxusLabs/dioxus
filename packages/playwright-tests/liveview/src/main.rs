// This test is used by playwright configured in the root of the repo

use axum::{
    Router,
    extract::{Query, ws::WebSocketUpgrade},
    response::Html,
    routing::get,
};
use dioxus::{logger::tracing::Level, prelude::*};
use futures_util::StreamExt;
use std::collections::HashMap;
use tower_http::cors::CorsLayer;

fn app() -> Element {
    let mut num = use_signal(|| 0);
    let mut submitted_files = use_signal(String::new);
    let mut description_values = use_signal(Vec::<String>::new);
    let mut upload_selected = use_signal(String::new);
    let mut large_upload = use_signal(String::new);
    let mut retained_files = use_signal(Vec::<dioxus::html::FileData>::new);

    rsx! {
        div {
            "hello axum! {num}"
            button { onclick: move |_| num += 1, "Increment" }
        }
        svg { circle { cx: 50, cy: 50, r: 40, stroke: "green", fill: "yellow" } }
        div { class: "raw-attribute-div", "raw-attribute": "raw-attribute-value" }
        div { class: "hidden-attribute-div", hidden: true }
        div {
            class: "dangerous-inner-html-div",
            dangerous_inner_html: "<p>hello dangerous inner html</p>"
        }
        input { id: "input-value", value: "hello input" }
        div { class: "style-div", color: "red", "colored text" }
        OnMounted {}
        FilePicker { id: "file-picker" }
        form {
            id: "upload-form",
            onsubmit: move |event| async move {
                upload_selected.set(format!("{:?}", event.get_first("uploads")));
                submitted_files.set(describe_files(event.files()).await);
            },
            input {
                name: "description",
                value: "upload description",
                oninput: move |event| {
                    description_values.write().push(event.value());
                    upload_selected.set(format!("{:?}", event.get_first("uploads")));
                },
            }
            FilePicker { id: "form-file-picker", name: "uploads" }
        }
        pre { id: "submitted-files", "{submitted_files}" }
        pre { id: "description-values", "{description_values.read().join(\",\")}" }
        pre { id: "upload-selected", "{upload_selected}" }
        input {
            id: "large-file-picker",
            r#type: "file",
            multiple: true,
            onchange: move |event| async move {
                let mut uploads = Vec::new();
                for file in event.files() {
                    uploads.push(describe_stream(file).await);
                }
                large_upload.set(uploads.join("\n"));
            },
        }
        pre { id: "large-upload", "{large_upload}" }
        form {
            id: "retained-form",
            onsubmit: move |event| retained_files.set(event.files()),
            input {
                id: "retained-file-picker",
                name: "files",
                r#type: "file",
                onchange: move |event| retained_files.set(event.files()),
            }
        }
        pre { id: "retained-files", "{retained_files.read().len()}" }
        button { onclick: move |_| retained_files.clear(), "Release files" }
    }
}

#[component]
fn FilePicker(id: &'static str, name: Option<&'static str>) -> Element {
    let mut input_files = use_signal(String::new);
    let mut change_files = use_signal(String::new);
    let mut value_files = use_signal(String::new);
    let mut description = use_signal(String::new);
    let mut text_contents = use_signal(String::new);
    let mut input_count = use_signal(|| 0);
    let mut change_count = use_signal(|| 0);

    rsx! {
        label { r#for: id, "Choose {id}" }
        input {
            id,
            name,
            r#type: "file",
            multiple: true,
            oninput: move |event| async move {
                input_count += 1;
                input_files.set(describe_files(event.files()).await);
            },
            onchange: move |event| async move {
                change_count += 1;
                if let Some(FormValue::Text(text)) = event.get_first("description") {
                    description.set(text);
                }
                let files = event.get(name.unwrap_or_default()).into_iter().filter_map(|value| {
                    match value {
                        FormValue::File(file) => file,
                        FormValue::Text(_) => None,
                    }
                }).collect();
                value_files.set(describe_files(files).await);
                if let Some(file) = event.files().first() {
                    text_contents.set(file.read_string().await.unwrap_or_default());
                }
                change_files.set(describe_files(event.files()).await);
            },
        }
        pre { id: "{id}-input", "{input_files}" }
        pre { id: "{id}-change", "{change_files}" }
        pre { id: "{id}-values", "{value_files}" }
        pre { id: "{id}-text", "{text_contents}" }
        span { id: "{id}-description", "{description}" }
        span { id: "{id}-counts", "{input_count},{change_count}" }
    }
}

async fn describe_stream(file: dioxus::html::FileData) -> String {
    let mut stream = file.byte_stream();
    let mut size = 0;
    let mut first = 0;
    let mut last = 0;
    while let Some(chunk) = stream.next().await {
        let bytes = match chunk {
            Ok(bytes) => bytes,
            Err(error) => return format!("{}|ERROR: {error}", file.name()),
        };
        assert!(bytes.len() <= 64 * 1024);
        if size == 0 {
            first = bytes.first().copied().unwrap_or_default();
        }
        last = bytes.last().copied().unwrap_or_default();
        size += bytes.len();
    }
    format!("{}|{size}|{first}|{last}", file.name())
}

async fn describe_files(files: Vec<dioxus::html::FileData>) -> String {
    let mut descriptions = Vec::new();
    for file in files {
        let contents = match file.read_bytes().await {
            Ok(bytes) => format!("{:?}", bytes.as_ref()),
            Err(error) => format!("ERROR: {error}"),
        };
        descriptions.push(format!(
            "{}|{}|{}|{}|{contents}",
            file.name(),
            file.size(),
            file.content_type().unwrap_or_default(),
            file.last_modified(),
        ));
    }
    descriptions.join("\n")
}

#[component]
fn OnMounted() -> Element {
    let mut mounted_triggered_count = use_signal(|| 0);
    rsx! {
        div {
            class: "onmounted-div",
            onmounted: move |_| {
                mounted_triggered_count += 1;
            },
            "onmounted was called {mounted_triggered_count} times"
        }
    }
}

#[tokio::main]
async fn main() {
    _ = dioxus::logger::init(Level::DEBUG);

    let addr: std::net::SocketAddr = ([127, 0, 0, 1], 3030).into();

    let view = dioxus_liveview::LiveViewPool::new();

    let websocket_view = view.clone();
    let app = Router::new()
        .route(
            "/",
            get(move |Query(options): Query<HashMap<String, String>>| async move {
                let query = if options.contains_key("direct") { "?direct=true" } else { "" };
                Html(format!(
                    r#"
            <!DOCTYPE html>
            <html>
                <head> <title>Dioxus LiveView with axum</title>  </head>
                <body> <div id="main"></div> </body>
                {glue}
            </html>
            "#,
                    glue = dioxus_liveview::interpreter_glue(&format!("ws://{addr}/ws{query}"))
                ))
            }),
        )
        .route(
            "/ws",
            get(move |ws: WebSocketUpgrade, Query(options): Query<HashMap<String, String>>| async move {
                let view = websocket_view.clone();
                ws.on_upgrade(move |socket| async move {
                    if options.contains_key("direct") {
                        _ = tokio::task::spawn_blocking(move || {
                            tokio::runtime::Handle::current().block_on(async move {
                                _ = view.run(VirtualDom::new(app), dioxus_liveview::axum_socket(socket)).await;
                            });
                        }).await;
                    } else {
                        _ = view.launch(dioxus_liveview::axum_socket(socket), app).await;
                    }
                })
            }),
        )
        .route(
            "/ws/upload/{token}",
            dioxus_liveview::axum_file_upload(view),
        )
        .layer(
            CorsLayer::new()
                .allow_origin("http://localhost:3030".parse::<axum::http::HeaderValue>().unwrap())
                .allow_methods([axum::http::Method::PUT])
                .allow_headers([
                    axum::http::header::CONTENT_TYPE,
                    axum::http::HeaderName::from_static("x-content-size"),
                ])
                .allow_credentials(true),
        );

    println!("Listening on http://{addr}");

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}
