//! This example showcases how to handle multipart form data uploads in Dioxus.
//!
//! Dioxus provides the `MultipartFormData` type to allow converting from the websys `FormData`
//! type directly into a streaming multipart form data handler.

use dioxus::{fullstack::MultipartFormData, prelude::*};

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    // The `MultipartFormData` type can be used to handle multipart form data uploads.
    // We can convert into it by using `.into()` on the `FormEvent`'s data, or by crafting
    // a `MultipartFormData` instance manually.
    let mut upload_as_multipart = use_action(move |event: FormEvent| upload(event.into()));

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
                label { r#for: "headshot", "Photos" }
                input { r#type: "file", name: "headshot", multiple: true, accept: ".png,.jpg,.jpeg" }
                label { r#for: "resume", "Resume" }
                input { r#type: "file", name: "resume", multiple: false, accept: ".pdf" }
                label { r#for: "name", "Name" }
                input { r#type: "text", name: "name", placeholder: "Name" }
                label { r#for: "age", "Age" }
                input { r#type: "number", name: "age", placeholder: "Age" }
                input { r#type: "submit", name: "submit", value: "Submit your resume" }
            }
        }
    }
}

/// Upload an entire form as multipart form data. This is useful when uploading multiple files
/// from a form.
///
/// MultipartFormData is typed over the form data structure, allowing us to extract
/// both files and other form fields in a type-safe manner.
///
/// On the server, we have access to axum's `Multipart` extractor
#[post("/api/upload-multipart")]
async fn upload(mut form: MultipartFormData) -> Result<()> {
    while let Ok(Some(field)) = form.next_field().await {
        info!("Got field: {:?}", field);
        info!(
            "Field:
            name: {:?}, filename: {:?}, content_type: {:?}",
            field.name().to_owned(),
            field.file_name(),
            field.content_type(),
        );
    }

    Ok(())
}
