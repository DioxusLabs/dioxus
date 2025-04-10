// Regression test for https://github.com/DioxusLabs/dioxus/pull/3958

use dioxus::prelude::*;

fn main() {
    launch(|| {
        rsx! {
            "Hello World!"
        }
    });
}

#[server(endpoint = "static_routes")]
async fn static_routes() -> Result<Vec<String>, ServerFnError> {
    Ok(vec!["/".to_string()])
}
