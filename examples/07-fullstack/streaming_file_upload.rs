//! This example showcases how to upload files from the client to the server.
//!
//! We can use the `FileStream` type to handle file uploads in a streaming fashion.
//! This allows us to handle large files without loading them entirely into memory.
//!
//! `FileStream` and `FileDownload` are built on multi-part form data and streams, which we
//! also showcase here.

use dioxus::{
    fullstack::{ByteStream, FileStream},
    prelude::*,
};
use dioxus_html::{FileData, HasFileData};
use futures::StreamExt;

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

    let mut download_file = use_action(move || async move {
        let mut file = download_as_filestream().await?;
        let mut bytes = vec![];

        info!("Downloaded file: {:?}", file);

        while let Some(Ok(chunk)) = file.next().await {
            bytes.extend_from_slice(&chunk);
        }

        dioxus::Ok(String::from_utf8_lossy(&bytes).to_string())
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
                    class: "drop-zone",
                    ondragover: move |evt| evt.prevent_default(),
                    ondrop: move |evt| async move {
                        evt.prevent_default();
                        upload_as_file_upload.call(evt.files()).await;
                    },
                    "Drop files here"
                }
                pre { "{upload_as_file_upload.value():?}" }
            }

            div {
                h3 { "Upload as ByteStream" }
                div {
                    class: "drop-zone",
                    ondragover: move |evt| evt.prevent_default(),
                    ondrop: move |evt| async move {
                        evt.prevent_default();
                        upload_files_as_bytestream.call(evt.files()).await;
                    },
                    "Drop files here"
                }
            }

            div {
                h3 { "Download a file from the server" }
                button { onclick: move |_| download_file.call(), "Download file" }
                if let Some(Ok(content)) = &download_file.value() {
                    pre { "{content}" }
                } else if let Some(Err(e)) = &download_file.value() {
                    pre { "Error downloading file: {e}" }
                }
            }
        }
    }
}

/// Upload a file using the `FileStream` type which automatically sets relevant metadata
/// as headers like Content-Type, Content-Length, and Content-Disposition.
#[post("/api/upload_as_file_stream")]
async fn upload_file_as_filestream(mut upload: FileStream) -> Result<u32> {
    use futures::StreamExt;
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

/// Upload a file as a raw byte stream. This requires us to specify the file name and size
/// as query parameters since the `ByteStream` type is an opaque stream without metadata.
///
/// We could also use custom headers to pass metadata if we wanted to avoid query parameters.
#[post("/api/upload_as_bytestream?name&size")]
async fn upload_as_bytestream(name: String, size: u64, mut stream: ByteStream) -> Result<()> {
    let mut collected = 0;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        collected += chunk.len() as u64;

        info!("Received {} bytes for file {}", chunk.len(), name);

        if collected > size {
            HttpError::bad_request("Received more data than expected")?;
        }
    }

    Ok(())
}

/// Download a file from the server as a `FileStream`. This automatically sets relevant
/// headers like Content-Type, Content-Length, and Content-Disposition.
///
/// This endpoint is nice because 3rd-party clients can visit it directly and download the file!
/// Try visiting this endpoint directly in your browser.
#[get("/api/download_as_filestream")]
async fn download_as_filestream() -> Result<FileStream> {
    Ok(FileStream::from_path(file!()).await?)
}
