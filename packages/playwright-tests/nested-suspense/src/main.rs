#![allow(non_snake_case)]
use dioxus::prelude::*;
use nested_suspense::app;

fn main() {
    LaunchBuilder::new()
        .with_cfg(server_only! {
            ServeConfig::builder()
                .enable_out_of_order_streaming()
        })
        .launch(app);
}
