use dioxus::prelude::*;

fn main() {
    dioxus::launch(app);
}

#[server]
async fn hello_world() -> Result<String, ServerFnError> {
    Ok("Hello, world!".to_string())
}

fn app() -> Element {
    let hello_world = use_server_future(hello_world)?;
    let hello_world = hello_world().unwrap().unwrap();
    rsx! {
        "Hello, world! {hello_world}"
    }
}
