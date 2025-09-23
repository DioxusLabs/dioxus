use dioxus::{
    fullstack::{KnownSize, RangeBody, RangeHeader},
    prelude::*,
};

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    rsx! {
        div {
            h1 { "Fullstack Video Stream Example" }
            video {
                src: "/api/video-stream/sample.mp4",
                controls: "true",
                width: "600",
            }
        }
    }
}

#[get("/api/video-stream/{filename}", range: RangeHeader)]
async fn stream_video(filename: String) -> Result<KnownSize<()>> {
    let path = "/Users/jonathankelley/Downloads/BigBuckBunny.mp4";

    let buffer = tokio::fs::read(path).await.unwrap();
    // processing data here...

    // let body = KnownSize::bytes(buffer);
    // let range = range.map(|TypedHeader(range)| range);
    // Ranged::new(range, body)

    todo!()
}
