//! This example shows how to use the `file` methods on FormEvent and DragEvent to handle file uploads and drops.
//!
//! Dioxus intercepts these events and provides a Rusty interface to the file data. Since we want this interface to
//! be crossplatform,

use std::sync::Arc;

use dioxus::prelude::*;
use dioxus::{html::HasFileData, prelude::dioxus_elements::FileEngine};

fn main() {
    LaunchBuilder::desktop().launch(app);
}

fn app() -> Element {
    let mut enable_directory_upload = use_signal(|| false);
    let mut files_uploaded = use_signal(|| Vec::new() as Vec<String>);

    let read_files = move |file_engine: Arc<dyn FileEngine>| async move {
        let files = file_engine.files();
        for file_name in &files {
            if let Some(file) = file_engine.read_file_to_string(file_name).await {
                files_uploaded.write().push(file);
            }
        }
    };

    let upload_files = move |evt: FormEvent| async move {
        if let Some(file_engine) = evt.files() {
            read_files(file_engine).await;
        }
    };

    let handle_file_drop = move |evt: DragEvent| async move {
        if let Some(file_engine) = evt.files() {
            read_files(file_engine).await;
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

        label { r#for: "directory-upload", "Enable directory upload" }

        input {
            r#type: "file",
            accept: ".txt,.rs,.js",
            multiple: true,
            name: "textreader",
            directory: enable_directory_upload,
            onchange: upload_files,
        }

        label { r#for: "textreader", "Upload text/rust files and read them" }

        div {
            // cheating with a little bit of JS...
            "ondragover": "this.style.backgroundColor='#88FF88';",
            "ondragleave": "this.style.backgroundColor='#FFFFFF';",
            "ondrop": "this.style.backgroundColor='#FFFFFF';",
            id: "drop-zone",
            ondrop: handle_file_drop,
            "Drop files here"
        }

        ul {
            for file in files_uploaded.read().iter() {
                li { "{file}" }
            }
        }
    }
}
