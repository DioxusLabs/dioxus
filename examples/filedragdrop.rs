use dioxus::events::DragEvent;
use dioxus::prelude::*;
use dioxus_desktop::DesktopConfig;

fn main() {
    dioxus_desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    cx.render(rsx!(
        div {
            h1 { "drag a file here and check your console" }
            div {
                height: "500px",
                width: "500px",
                background: "red",
                ondragenter: move |evt: DragEvent| {
                    println!("drag enter {:?}", evt.files());
                },
                ondragleave: move |_| {
                    println!("drag leave");
                }
            }
        }
    ))
}
