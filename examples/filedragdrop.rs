use dioxus::events::DragEvent;
use dioxus::prelude::*;

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
            input {
                "type": "file",
                multiple: "true",
                oninput: move |evt| println!("input event: {:?}", evt),
            }
            div {
                id: "dropzone",
                height: "500px",
                width: "500px",
                background: "red",
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
