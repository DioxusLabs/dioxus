mod dioxus_demo;
mod loop_demo;
mod tui_demo;
mod ws_conn;

fn main() -> anyhow::Result<()> {
    ws_conn::initialize();

    let demo = std::env::var("DEMO").unwrap_or("dioxus".to_string());

    match demo.as_str() {
        "dioxus" => dioxus_demo::launch(),
        "loop" => loop_demo::launch(),
        "tui" => tui_demo::launch(),
        _ => Err(anyhow::anyhow!("Unknown demo: {}", demo)),
    }
}
