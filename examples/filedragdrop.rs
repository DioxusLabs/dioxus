use dioxus::prelude::*;
use dioxus_desktop::Config;

fn main() {
    LaunchBuilder::new(app)
        .cfg(Config::new().with_file_drop_handler(|_w, e| {
            println!("{e:?}");
            true
        }))
        .launch()
}

fn app() -> Element {
    rsx!(
        div { h1 { "drag a file here and check your console" } }
    )
}
