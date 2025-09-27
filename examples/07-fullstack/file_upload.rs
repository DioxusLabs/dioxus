use bytes::Bytes;
use dioxus::{prelude::*, server::axum::extract::Multipart};
use dioxus_fullstack::FileUpload;

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    let mut file_id = use_action(move || async move {
        // let file = FileUpload::from_stream(
        //     "myfile.png".to_string(),
        //     futures::stream::iter(vec![
        //         Bytes::from_static(b"hello"),
        //         Bytes::from_static(b"world"),
        //     ]),
        // );

        // reqwest::multipart::Form::new()

        // upload_file(file).await

        dioxus::Ok(todo!())
    });

    let onsubmit = move |evt: FormEvent| {
        info!("Form submitted!");
        for file in evt.files() {
            info!("File: {:?}", file);
        }
    };

    rsx! {
        div { "File upload example" }
        input { r#type: "file", id: "file", name: "file", multiple: true, accept: ".png,.jpg,.jpeg", oninput: onsubmit, }
        label { r#for: "file", "File Upload" }
    }
}

#[post("/api/upload_image")]
async fn upload_file() -> Result<u32> {
    // async fn upload_file(mut upload: FileUpload) -> Result<u32> {
    // async fn upload_file(mut upload: Multipart) -> Result<u32> {
    use std::env::temp_dir;
    // let uploade_dir = temp_dir().join("uploads");

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

// server::axum::extract::Multipart,
// /// In our `login` form, we'll return a `SetCookie` header if the login is successful.
// ///
// /// This will set a cookie in the user's browser that can be used for subsequent authenticated requests.
// /// The `SetHeader::new()` method takes anything that can be converted into a `HeaderValue`.
// ///
// /// We can set multiple headers by returning a tuple of `SetHeader` types, or passing in a tuple
// /// of headers to `SetHeader::new()`.
// #[post("/api/login")]
// async fn login(mut form: Multipart) -> Result<()> {
//     while let Some(mut field) = form.next_field().await.unwrap() {
//         let name = field.name().unwrap().to_string();
//         let data = field.bytes().await.unwrap();

//         println!("Length of `{}` is {} bytes", name, data.len());
//     }

//     todo!()
// }
