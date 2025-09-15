use bytes::Bytes;
use dioxus::prelude::*;
use dioxus_fullstack::FileUpload;

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    let mut file_id = use_action(move |()| make_req(123, "a".into(), "b".into(), "c".into()));

    rsx! {
        div { "Access to full axum request" }
        button { onclick: move |_| file_id.dispatch(()), "Upload file" }
    }
}

#[post("/api/full_request_access/{id}/?a&b&c", request: axum::extract::Request)]
async fn make_req(id: u32, a: String, b: String, c: String) -> Result<u32> {
    // use std::env::temp_dir;
    // let target_file = temp_dir().join("uploads").join("myfile.png");
    todo!()
}
