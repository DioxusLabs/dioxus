#![allow(non_snake_case)]
use dioxus::prelude::*;
use nested_suspense::app;

fn main() {
    dioxus::LaunchBuilder::new()
        .with_cfg(server_only! {
            ServeConfig::builder()
                .incremental(
                    IncrementalRendererConfig::new()
                        .static_dir(
                            std::env::current_exe()
                                .unwrap()
                                .parent()
                                .unwrap()
                                .join("public")
                        )
                )
                .enable_out_of_order_streaming()
        })
        .launch(app);
}

#[server(endpoint = "static_routes")]
async fn static_routes() -> Result<Vec<String>, ServerFnError> {
    Ok(vec!["/".to_string()])
}
