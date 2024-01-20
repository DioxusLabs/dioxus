use dioxus::desktop::Config;
use dioxus::prelude::*;

fn main() {
    LaunchBuilder::desktop()
        .with_cfg(Config::new().with_file_drop_handler(|_w, e| {
            println!("{e:?}");
            true
        }))
        .launch(app)
}

fn app() -> Element {
    rsx!(
        div { h1 { "drag a file here and check your console" } }
    )
}
