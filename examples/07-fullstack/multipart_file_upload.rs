//! This example showcases how to upload files from the client to the server.
//!
//! We can use the `FileStream` type to handle file uploads in a streaming fashion.
//! This allows us to handle large files without loading them entirely into memory.
//!
//! `FileStream` and `FileDownload` are built on multi-part form data and streams, which we
//! also showcase here.

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
    // Dioxus provides the `FileStream` type for efficiently uploading files in a streaming fashion.
    // This approach automatically automatically sets relevant metadata such as headers like
    // Content-Type, Content-Length, and Content-Disposition.
    //
    // The `FileStream` type can be created from a `FileData` instance using `.into()`.
    // This approach is better suited for public-facing APIs where standard headers are expected.
    //
    // `FileStream` uses the platform's native file streaming capabilities when available,
    // making it more efficient than manually streaming bytes.
    let mut upload_as_file_upload = use_action(move |files: Vec<FileData>| async move {
        for file in files {
            upload_file_as_filestream(file.into()).await?;
        }
        dioxus::Ok(())
    });

    // If we have multiple files to upload, we can use multipart form data which also uses streaming
    // The `FormData` type from dioxus-html automatically implements `Into<MultipartData>` for us,
    // making it possible to use multipart form data without converting each file individually.
    let mut upload_as_multipart = use_action(move |data: LoginFormData| async move {
        // todo!()
        // for file in files {
        //     let res = upload_file(file.into()).await;
        // }
        dioxus::Ok(())
    });

    // We can upload files by directly using the `ByteStream` type. With this approach, we need to
    // specify the file name and size as query parameters since its an opaque stream.
    //
    // The `FileData` type has a `byte_stream` method which returns a `Pin<Box<dyn Stream<Item = Bytes> + Send>>`
    // that we can turn into a `ByteStream` with `.into()`.
    //
    // In WASM, this will buffer the entire file in memory, so it's not the most efficient way to upload files.
    // This approach is best suited for data created by the user in the browser.
    let mut upload_files_as_bytestream = use_action(move |files: Vec<FileData>| async move {
        info!("Uploading {} files", files.len());
        for file in files {
            upload_as_bytestream(file.name(), file.size(), file.byte_stream().into()).await?;
        }
        dioxus::Ok(())
    });

    rsx! {
        Stylesheet { href: asset!("/examples/assets/file_upload.css") }
        div {
            max_width: "600px",
            margin: "auto",
            h1 { "File upload example" }
            div {
                h3 { "Upload as FileUpload" }
                div {
                    height: "100px",
                    background_color: "lightgray",
                    ondragover: move |evt| evt.prevent_default(),
                    ondrop: move |evt| async move {
                        evt.prevent_default();
                        upload_as_file_upload.call(evt.files()).await;
                    },
                    "Drop files here"
                }
                pre { "{upload_as_file_upload.result():?}" }
            }


            div {
                h3 { "Upload as Multipart" }
                p { "Use the built-in multipart form handling" }
                form {
                    display: "flex",
                    flex_direction: "column",
                    gap: "8px",
                    onsubmit: move |evt| async move {
                        evt.prevent_default();
                        // upload_as_multipart.call(evt.data().into()).await;
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

            div {
                h3 { "Upload as ByteStream" }
                div {
                    id: "drop-zone",
                    background_color: "lightgray",
                    ondragover: move |evt| evt.prevent_default(),
                    ondrop: move |evt| async move {
                        evt.prevent_default();
                        upload_files_as_bytestream.call(evt.files()).await;
                    },
                    "Drop files here"
                }
            }
        }
    }
}

/// Upload a file using the `FileStream` type which automatically sets relevant metadata
/// as headers like Content-Type, Content-Length, and Content-Disposition.
#[post("/api/upload_as_file_stream")]
async fn upload_file_as_filestream(mut upload: FileUpload) -> Result<u32> {
    use std::env::temp_dir;
    use tokio::io::AsyncWriteExt;

    info!("Received file upload: {:?}", upload);

    // Create a temporary file to write the uploaded data to.
    let upload_file = std::path::absolute(temp_dir().join(upload.file_name()))?;

    // Reject paths that are outside the temp directory for security reasons.
    if !upload_file.starts_with(temp_dir()) {
        HttpError::bad_request("Invalid file path")?;
    }

    info!(
        "Uploading bytes of {:?} file to {:?}",
        upload.size(),
        upload_file
    );

    // Open the file for writing.
    tokio::fs::create_dir_all(upload_file.parent().unwrap()).await?;
    let mut file = tokio::fs::File::create(&upload_file).await?;
    let expected = upload.size();

    // Stream the data from the request body to the file.
    let mut uploaded: u64 = 0;
    let mut errored = false;
    while let Some(chunk) = upload.next().await {
        match chunk {
            Ok(bytes) => {
                uploaded += bytes.len() as u64;
                if file.write_all(&bytes).await.is_err() {
                    errored = true;
                    break;
                }

                // 1GB max file size or attempting to upload more than expected.
                if uploaded > expected.unwrap_or(1024 * 1024 * 1024) {
                    errored = true;
                    break;
                }
            }
            Err(_) => {
                errored = true;
                break;
            }
        }
    }

    // Clean up the file if there was an error during upload.
    if errored {
        _ = file.sync_data().await;
        let _ = tokio::fs::remove_file(&upload_file).await;
        HttpError::internal_server_error("Failed to upload file")?;
    }

    Ok(uploaded as u32)
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

/// Upload a file as a raw byte stream. This requires us to specify the file name and size
/// as query parameters since the `ByteStream` type is an opaque stream without metadata.
#[post("/api/upload_as_bytestream?name&size")]
async fn upload_as_bytestream(name: String, size: u64, mut stream: ByteStream) -> Result<()> {
    todo!()
}
