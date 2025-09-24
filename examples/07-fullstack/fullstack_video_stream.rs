//! This example showcases how to use the `RangedBytes` response type to serve
//! video files with support for HTTP range requests.
//!
//! `RangedBytes` can be constructed from either a file, an in-memory buffer, or a stream that
//! implements `AsyncRead + AsyncSeekStart` - two traits exposed by `dioxus-fullstack`.
//!
//! The `RangedBytes` type is currently not very useful on the client side, so it's mostly meant
//! to allow serving of large static assets like videos or large files.

use dioxus::{fullstack::RangedBytes, prelude::*};

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    rsx! {
        div {
            h1 { "Fullstack Video Stream Example" }
            video {
                src: "/api/video-stream/big-buck-bunny.mp4",
                autoplay: true,
                controls: true,
                width: 640,
                height: 480
            }
        }
    }
}

#[get("/api/video-stream/big-buck-bunny.mp4", range: dioxus::fullstack::RangeHeader)]
async fn stream_video() -> Result<RangedBytes> {
    let path = std::path::PathBuf::from("./examples/assets/test_video.mp4");

    if !path.exists() {
        panic!(
            "make sure to run the `video_stream` desktop example first to download the sample video"
        );
    }

    Ok(RangedBytes::from_file(path, range).await?)
}
