use axum::{extract::State, response::Html, Json};
use dioxus::prelude::*;
use dioxus_fullstack::{route, DioxusServerState, ServerFunction};

#[tokio::main]
async fn main() {
    ServerFunction::serve(|| {
        let routes = ServerFunction::collect();

        rsx! {
            h1 { "We have dioxus fullstack at home!" }
            div { "Our routes:" }
            ul {
                for r in routes {
                    li {
                        a { href: "{r.path()}", "{r.method()} {r.path()}" }
                    }
                }
                button {
                    onclick: move |_| async move {
                        // let res = get_item(1, None, None).await?;
                    }
                }
            }
        }
    })
    .await;
}

#[get("/home")]
async fn home(state: State<DioxusServerState>) -> String {
    format!("hello home!")
}

#[get("/home/{id}")]
async fn home_page(id: i32) -> String {
    format!("hello home {}", id)
}

#[get("/item/{id}?amount&offset")]
async fn get_item(id: i32, amount: Option<i32>, offset: Option<i32>) -> Json<YourObject> {
    Json(YourObject { id, amount, offset })
}

#[derive(serde::Serialize, serde::Deserialize)]
struct YourObject {
    id: i32,
    amount: Option<i32>,
    offset: Option<i32>,
}

#[post("/work")]
async fn post_work() -> Html<&'static str> {
    Html("post work")
}

#[get("/work")]
async fn get_work() -> Html<&'static str> {
    Html("get work")
}

#[get("/play")]
async fn go_play() -> Html<&'static str> {
    Html("hello play")
}

#[get("/dx-element")]
async fn get_element() -> Html<String> {
    Html(dioxus_ssr::render_element(rsx! {
        div { "we have ssr at home..." }
    }))
}
