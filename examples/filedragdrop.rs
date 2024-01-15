use dioxus::prelude::*;
use dioxus_desktop::Config;

fn main() {
    Config::new()
        .with_file_drop_handler(|_w, e| {
            println!("{e:?}");
            true
        })
        .launch(app)
}

fn app(_props: ()) -> Element {
    rsx!(
        div {
            h1 { "drag a file here and check your console" }
        }
    )
}
