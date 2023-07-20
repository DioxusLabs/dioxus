#![allow(non_snake_case)]
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
                    enable_directory_upload.set(evt.value.parse().unwrap());
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
                    if let Some(file_engine) = &evt.files {
                        let files = file_engine.files();
                        for file_name in files {
                            sleep(std::time::Duration::from_secs(1)).await;
                            files_uploaded.write().push(file_name);
                        }
                    }
                }
            },
        },

        div { "progress: {files_uploaded.read().len()}" },

        ul {
            for file in files_uploaded.read().iter() {
                li { "{file}" }
            }
        }
    })
}
