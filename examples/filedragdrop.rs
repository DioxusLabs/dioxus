use dioxus::prelude::*;
use dioxus_desktop::DesktopConfig;

fn main() {
    let cfg = DesktopConfig::new().with_file_drop_handler(|_w, e| {
        println!("{:?}", e);
        true
    });

    dioxus_desktop::launch_with_props(app, (), cfg);
}

fn app(cx: Scope) -> Element {
    cx.render(rsx!(
        div {
            h1 { "drag a file here and check your console" }
        }
    ))
}
