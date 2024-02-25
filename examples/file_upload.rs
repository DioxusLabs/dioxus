//! This example shows how to use the `file` methods on FormEvent and DragEvent to handle file uploads and drops.
//!
//! Dioxus intercepts these events and provides a Rusty interface to the file data. Since we want this interface to
//! be crossplatform,

use dioxus::html::HasFileData;
use dioxus::prelude::*;
use tokio::time::sleep;

fn main() {
    launch(app);
}

fn app() -> Element {
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

    rsx! {
        style { {include_str!("./assets/file_upload.css")} }

        input {
            r#type: "checkbox",
            id: "directory-upload",
            checked: enable_directory_upload,
            oninput: move |evt| enable_directory_upload.set(evt.checked()),
        },
        label {
            r#for: "directory-upload",
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
            // cheating with a little bit of JS...
            "ondragover": "this.style.backgroundColor='#88FF88';",
            "ondragleave": "this.style.backgroundColor='#FFFFFF';",

            id: "drop-zone",
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
