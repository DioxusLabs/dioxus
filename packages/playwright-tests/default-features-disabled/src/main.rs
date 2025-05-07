// This test asserts that the client feature is disable on the server build by the cli
// even if it is set as a default feature

#![allow(non_snake_case)]
use dioxus::prelude::*;

fn main() {
    launch(app);
}

fn app() -> Element {
    let server_features = use_server_future(get_server_features)?.unwrap().unwrap();
    let mut client_features = use_signal(Vec::new);

    use_effect(move || {
        client_features.set(current_platform_features());
    });

    let mut count = use_signal(|| 0);

    rsx! {
        div {
            "server features: {server_features:?}"
        }
        div {
            "client features: {client_features:?}"
        }
        button {
            onclick: move |_| count += 1,
            "{count}"
        }
    }
}

fn current_platform_features() -> Vec<String> {
    let mut features = Vec::new();
    if cfg!(feature = "web") {
        features.push("web".to_string());
    }
    if cfg!(feature = "server") {
        features.push("server".to_string());
    }
    features
}

#[server]
async fn get_server_features() -> Result<Vec<String>, ServerFnError> {
    Ok(current_platform_features())
}
