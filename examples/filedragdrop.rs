use dioxus::prelude::*;
use dioxus_desktop::Config;

fn main() {
    let cfg = Config::new().with_file_drop_handler(|_w, e| {
        println!("{e:?}");
        true
    });

    dioxus_desktop::launch_with_props(app, (), cfg);
}

fn app() -> Element {
    rsx!(
        div {
            h1 { "drag a file here and check your console" }
        }
    )
}
