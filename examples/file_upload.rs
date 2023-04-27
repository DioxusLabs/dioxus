#![allow(non_snake_case)]
use dioxus::prelude::*;

fn main() {
    dioxus_desktop::launch(app);
}

fn app(cx: Scope) -> Element {
    let files_uploaded: &UseRef<Vec<String>> = use_ref(cx, Vec::new);

    cx.render(rsx! {
        input {
            r#type: "file",
            accept: ".txt, .rs",
            multiple: true,
            onchange: |evt| {
                to_owned![files_uploaded];
                async move {
                    if let Some(file_engine) = &evt.files {
                        let files = file_engine.files();
                        for file_name in &files {
                            if let Some(file) = file_engine.read_file_to_string(file_name).await{
                                files_uploaded.write().push(file);
                            }
                        }
                    }
                }
            },
        }
        div {
            width: "100px",
            height: "100px",
            border: "1px solid black",
            prevent_default: "ondrop dragover dragenter",
            ondrop: move |evt| {
                to_owned![files_uploaded];
                async move {
                    if let Some(file_engine) = &evt.files {
                        let files = file_engine.files();
                        for file_name in &files {
                            if let Some(file) = file_engine.read_file_to_string(file_name).await{
                                files_uploaded.write().push(file);
                            }
                        }
                    }
                }
            },
            ondragover: move |event: DragEvent| {
                event.stop_propagation();
            },
            "Drop files here"
        }

        ul {
            for file in files_uploaded.read().iter() {
                li { "{file}" }
            }
        }
    })
}
