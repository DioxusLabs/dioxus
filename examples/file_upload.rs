#![allow(non_snake_case)]
use dioxus::html::HasFileData;
use dioxus::prelude::*;
use tokio::time::sleep;

fn main() {
    launch_desktop(App);
}

fn App() -> Element {
    let mut enable_directory_upload = use_signal(|| false);
    let mut files_uploaded = use_signal(|| Vec::new() as Vec<String>);

    let upload_files = move |evt: FormEvent| async move {
        for file_name in evt.files().unwrap().files() {
            // no files on form inputs?
            sleep(std::time::Duration::from_secs(1)).await;
            files_uploaded.write().push(file_name);
        }
    };

    let handle_file_drop = move |evt: DragEvent| async move {
        if let Some(file_engine) = &evt.files() {
            let files = file_engine.files();
            for file_name in &files {
                if let Some(file) = file_engine.read_file_to_string(file_name).await {
                    files_uploaded.write().push(file);
                }
            }
        }
    };

    render! {
        label {
            input {
                r#type: "checkbox",
                checked: enable_directory_upload,
                oninput: move |evt| enable_directory_upload.set(evt.value().parse().unwrap()),
            },
            "Enable directory upload"
        }

        input {
            r#type: "file",
            accept: ".txt,.rs",
            multiple: true,
            directory: enable_directory_upload,
            onchange: upload_files,
        }
        div {
            width: "100px",
            height: "100px",
            border: "1px solid black",
            prevent_default: "ondrop dragover dragenter",
            ondrop: handle_file_drop,
            ondragover: move |event| event.stop_propagation(),
            "Drop files here"
        }

        ul {
            for file in files_uploaded.read().iter() {
                li { "{file}" }
            }
        }
    }
}
