//! This example showcases how to handle multipart form data uploads in Dioxus.

use async_std::prelude::StreamExt;
use bytes::Bytes;
use dioxus::{
    fullstack::{
        ByteStream, DioxusServerState, ExtractRequest, FileStream, MultipartStream, ServerFnEncoder,
    },
    prelude::*,
};
use dioxus_fullstack::FileUpload;
use dioxus_html::{FileData, HasFileData};

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    // If we have multiple files to upload, we can use multipart form data which also uses streaming
    // The `FormData` type from dioxus-html automatically implements `Into<MultipartData>` for us,
    // making it possible to use multipart form data without converting each file individually.
    let mut upload_as_multipart = use_action(move |data: FormEvent| async move {
        // todo!()
        // for file in files {
        //     let res = upload_file(file.into()).await;
        // }
        dioxus::Ok(())
    });

    rsx! {
        Stylesheet { href: asset!("/examples/assets/file_upload.css") }
        div {
            h3 { "Upload as Multipart" }
            p { "Use the built-in multipart form handling" }
            form {
                display: "flex",
                flex_direction: "column",
                gap: "8px",
                onsubmit: move |evt| async move {
                    evt.prevent_default();
                    upload_as_multipart.call(evt).await;
                },
                input { r#type: "file", name: "headshot", multiple: true, accept: ".png,.jpg,.jpeg" }
                label { r#for: "headshot", "Photos" }
                input { r#type: "file", name: "resume", multiple: false, accept: ".pdf" }
                label { r#for: "resume", "Resume" }
                input{ r#type: "text", name: "name", placeholder: "Name" }
                label { r#for: "name", "Name" }
                input{ r#type: "number", name: "age", placeholder: "Age" }
                label { r#for: "age", "Age" }
                input { type: "button", name: "submit", value: "Submit your resume"}
            }
        }
    }
}

struct LoginFormData {
    name: String,
    age: u32,
    resumee: FileData,
    photos: Vec<FileData>,
}

/// Upload an entire form as multipart form data. This is useful when uploading multiple files
/// from a form.
///
/// MultipartStream is typed over the form data structure, allowing us to extract
/// both files and other form fields in a type-safe manner.
async fn upload_as_multipart(data: MultipartStream<LoginFormData>) -> Result<u32> {
    // use std::env::temp_dir;
    // let uploade_dir = temp_dir().join("uploads");

    // match chunk.next_chunk() {
    //     LoginFormData::name(name) => {}
    //     LoginFormData::age(age) => {}
    //     LoginFormData::resumee(resumee) => {}
    //     LoginFormData::photos(photos) => {}
    //     _ => HttpError::bad_request("Invalid form data")?,
    // }

    // while let Some(chunk) = upload.next_chunk().await {
    //     // Write the chunk to the target file
    // }

    // while let Some(mut field) = upload.next_field().await.unwrap() {
    //     let name = field.name().unwrap().to_string();
    //     let data = field.bytes().await.unwrap();

    //     println!("Length of `{}` is {} bytes", name, data.len());
    // }

    todo!()
}
