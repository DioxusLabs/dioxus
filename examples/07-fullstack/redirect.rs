//! This example shows how to use the axum `Redirect` type to redirect the client to a different URL.
//!
//! On the web, a redirect will not be handled directly by JS, but instead the browser will automatically
//! follow the redirect. This is useful for redirecting to different pages after a form submission.

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
    rsx! {
        div {
            h1 { "Redirect Example" }
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
