#![allow(non_snake_case)]
use dioxus::prelude::*;
use tokio::time::sleep;

fn main() {
    dioxus_desktop::launch(App);
}

fn App(cx: Scope) -> Element {
    let files_uploaded: &UseRef<Vec<String>> = use_ref(cx, Vec::new);

    cx.render(rsx! {
        input {
            r#type: "file",
            accept: ".txt, .rs",
            multiple: true,
            directory: true,
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
