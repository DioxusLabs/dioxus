mod dioxus_demo;
mod loop_demo;
mod tui_demo;
mod ws_conn;

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    ws_conn::initialize();

    let demo = std::env::var("DEMO").unwrap_or("dioxus".to_string());

    match demo.as_str() {
        "dioxus" => dioxus_demo::launch(),
        "loop" => loop_demo::launch(),
        "tui" => tui_demo::launch(),
        _ => panic!("Unknown demo: {}", demo),
    }
}

#[cfg(target_arch = "wasm32")]
fn main() {
    dioxus_demo::launch();
}
