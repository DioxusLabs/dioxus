use bytes::Bytes;
use dioxus::prelude::*;
use dioxus_fullstack::FileUpload;

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    let mut file_id = use_action(move || async move {
        let file = FileUpload::from_stream(
            "myfile.png".to_string(),
            futures::stream::iter(vec![
                Bytes::from_static(b"hello"),
                Bytes::from_static(b"world"),
            ]),
        );

        upload_file(file).await
    });

    rsx! {
        div { "File upload example" }
        button { onclick: move |_| file_id.call(), "Upload file" }
    }
}

#[post("/api/upload_image")]
async fn upload_file(upload: FileUpload) -> Result<u32> {
    use std::env::temp_dir;
    let target_file = temp_dir().join("uploads").join("myfile.png");

    // while let Some(chunk) = upload.next_chunk().await {
    //     // Write the chunk to the target file
    // }

    todo!()
}
