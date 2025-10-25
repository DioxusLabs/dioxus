//! This example shows how to use the `file` methods on FormEvent and DragEvent to handle file uploads and drops.
//!
//! Dioxus intercepts these events and provides a Rusty interface to the file data. Since we want this interface to
//! be crossplatform,

use dioxus::html::HasFileData;
use dioxus::prelude::*;
use dioxus_html::FileData;

const STYLE: Asset = asset!("/examples/assets/file_upload.css");

fn main() {
    dioxus::launch(app);
}

struct UploadedFile {
    name: String,
    contents: String,
}

fn app() -> Element {
    let mut enable_directory_upload = use_signal(|| false);
    let mut files_uploaded = use_signal(|| Vec::new() as Vec<UploadedFile>);
    let mut hovered = use_signal(|| false);

    let upload_files = move |files: Vec<FileData>| async move {
        for file in files {
            let filename = file.name();
            if let Ok(contents) = file.read_string().await {
                files_uploaded.push(UploadedFile {
                    name: filename,
                    contents,
                });
            } else {
                files_uploaded.push(UploadedFile {
                    name: filename,
                    contents: "Failed to read file".into(),
                });
            }
        }
    };

    rsx! {
        Stylesheet { href: STYLE }

        h1 { "File Upload Example" }
        p { "Drop a .txt, .rs, or .js file here to read it" }
        button { onclick: move |_| files_uploaded.clear(), "Clear files" }

        div {
            label { r#for: "directory-upload", "Enable directory upload" }
            input {
                r#type: "checkbox",
                id: "directory-upload",
                checked: enable_directory_upload,
                oninput: move |evt| enable_directory_upload.set(evt.checked()),
            }
        }

        div {
            label { r#for: "textreader", "Upload text/rust files and read them" }
            input {
                r#type: "file",
                accept: ".txt,.rs,.js",
                multiple: true,
                name: "textreader",
                directory: enable_directory_upload,
                onchange: move |evt| async move {
                    upload_files(evt.files()).await
                },
            }
        }

        div {
            id: "drop-zone",
            background_color: if hovered() { "lightblue" } else { "lightgray" },
            ondragover: move |evt| {
                evt.prevent_default();
                hovered.set(true)
            },
            ondragleave: move |_| hovered.set(false),
            ondrop: move |evt| async move {
                evt.prevent_default();
                hovered.set(false);
                upload_files(evt.files()).await;
            },
            "Drop files here"
        }

        ul {
            for file in files_uploaded.read().iter().rev() {
                li {
                    span { "{file.name}" }
                    pre  { "{file.contents}"  }
                }
            }
        }
    }
}
