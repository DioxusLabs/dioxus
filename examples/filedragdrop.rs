use dioxus::prelude::*;

fn main() {
    dioxus_desktop::launch_with_props(app, (), |c| {
        c.with_file_drop_handler(|_w, e| {
            println!("{:?}", e);
            true
        })
    });
}

fn app(cx: Scope) -> Element {
    cx.render(rsx!(
        div {
            h1 { "drag a file here and check your console" }
        }
    ))
}
