use dioxus::prelude::*;
use futures_util::{future, stream, Stream, StreamExt};
use std::time::Duration;

fn main() {
    launch_desktop(app);
}

fn app() -> Element {
    let mut count = use_signal(|| 10);

    use_future(|| async move {
        let mut stream = some_stream();

        while let Some(second) = stream.next().await {
            count.set(second);
        }
    });

    rsx! {
        h1 { "High-Five counter: {count}" }
    }
}

fn some_stream() -> std::pin::Pin<Box<dyn Stream<Item = i32>>> {
    Box::pin(
        stream::once(future::ready(0)).chain(stream::iter(1..).then(|second| async move {
            tokio::time::sleep(Duration::from_secs(1)).await;
            second
        })),
    )
}
