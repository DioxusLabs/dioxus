//! Handle async streams using use_future and awaiting the next value.

use async_std::task::sleep;
use dioxus::prelude::*;
use futures_util::{future, stream, Stream, StreamExt};

fn main() {
    launch(app);
}

fn app() -> Element {
    let mut count = use_signal(|| 10);

    use_future(move || async move {
        // Create the stream.
        // This could be a network request, a file read, or any other async operation.
        let mut stream = some_stream();

        // Await the next value from the stream.
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
            sleep(std::time::Duration::from_secs(1)).await;
            second
        })),
    )
}
