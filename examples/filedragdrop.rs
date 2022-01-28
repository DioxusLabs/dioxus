use dioxus::prelude::*;

fn main() {
    dioxus::desktop::launch_with_props(app, (), |c| {
        c.with_file_drop_handler(|_w, e| {
            println!("{:?}", e);
            false
        })
    });
}

fn app(cx: Scope) -> Element {
    cx.render(rsx!(
        div {
            h1 { "drag an file here" }
        }
    ))
}
