#![allow(non_snake_case)]
use dioxus::html::HasFileData;
use dioxus::prelude::*;
use tokio::time::sleep;

fn main() {
    dioxus_desktop::launch(App);
}

fn App(cx: Scope) -> Element {
    let enable_directory_upload = use_state(cx, || false);
    let files_uploaded: &UseRef<Vec<String>> = use_ref(cx, Vec::new);

    cx.render(rsx! {
        label {
            input {
                r#type: "checkbox",
                checked: "{enable_directory_upload}",
                oninput: move |evt| {
                    enable_directory_upload.set(evt.value().parse().unwrap());
                },
            },
            "Enable directory upload"
        }

        input {
            r#type: "file",
            accept: ".txt,.rs",
            multiple: true,
            directory: **enable_directory_upload,
            onchange: |evt| {
                to_owned![files_uploaded];
                async move {
                    if let Some(file_engine) = &evt.files() {
                        let files = file_engine.files();
                        for file_name in files {
                            sleep(std::time::Duration::from_secs(1)).await;
                            files_uploaded.write().push(file_name);
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
                    if let Some(file_engine) = &evt.files() {
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
