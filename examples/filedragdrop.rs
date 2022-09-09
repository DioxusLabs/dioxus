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
                height: "250px",
                width: "250px",
                background: "green",
                draggable: "true",
            }
            div {
                id: "dropzone",
                height: "500px",
                width: "500px",
                background: "red",
                // prevent_default: "ondragover",
                // prevent_default: "ondragleave",
                // prevent_default: "ondragenter",
                // "ondragover": "return false;",
                ondragover: move |evt| {},
                ondragenter: move |evt: DragEvent| {
                    println!("drag enter {:?}", evt.files());
                },
                ondragleave: move |_| {
                    println!("drag leave");
                },
                ondrop: move |evt| {
                    println!("drop {:?}", evt.files());
                },
            }
        }
    ))
}
