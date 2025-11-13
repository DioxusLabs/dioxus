//! This example shows how to use the `Streaming<T, E>` type to send streaming responses from the
//! server to the client (and the client to the server!).
//!
//! The `Streaming<T, E>` type automatically coordinates sending and receiving streaming data over HTTP.
//! The `T` type parameter is the type of data being sent, and the `E` type parameter is the encoding
//! used to serialize and deserialize the data.
//!
//! Dioxus Fullstack provides several built-in encodings:
//! - JsonEncoding: the default, uses JSON for serialization
//! - CborEncoding: uses CBOR for binary serialization
//! - PostcardEncoding: uses Postcard for binary serialization
//! - MsgPackEncoding: uses MessagePack for binary serialization
//! - RkyvEncoding: uses Rkyv for zero-copy binary serialization
//!
//! The default encoding is `JsonEncoding`, which works well for most use cases and can be used by
//! most clients. If you need a more efficient binary encoding, consider using one of the
//! binary encodings.

use bytes::Bytes;
use dioxus::{
    fullstack::{JsonEncoding, Streaming, TextStream},
    prelude::*,
};
use futures::{StreamExt as _, TryStreamExt};

fn main() {
    dioxus::launch(app)
}

fn app() -> Element {
    let mut text_responses = use_signal(String::new);
    let mut json_responses = use_signal(Vec::new);
    let mut echo_responses = use_signal(Vec::new);
    let mut transform_responses = use_signal(Vec::new);

    let mut start_text_stream = use_action(move || async move {
        text_responses.clear();
        let mut stream = text_stream(Some(100)).await?;

        while let Some(Ok(text)) = stream.next().await {
            text_responses.push_str(&text);
            text_responses.push('\n');
        }

        dioxus::Ok(())
    });

    let mut start_json_stream = use_action(move || async move {
        json_responses.clear();
        let mut stream = json_stream().await?;

        while let Some(Ok(dog)) = stream.next().await {
            json_responses.push(dog);
        }

        dioxus::Ok(())
    });

    let mut continue_echo_stream = use_signal_sync(|| false);
    let mut start_echo_stream = use_action(move || async move {
        continue_echo_stream.set(true);
        echo_responses.clear();
        let stream = echo_stream(Streaming::new(
            futures::stream::unfold(0, move |index| async move {
                if !continue_echo_stream() {
                    return None;
                }
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                let dog = Dog {
                    name: format!("Dog {}", index),
                    age: (index % 10) as u8,
                };
                Some((dog, index + 1))
            }),
        ))
        .await?;
        stream
            .into_inner()
            .try_for_each(move |dog| async move {
                echo_responses.push(dog);
                Ok(())
            })
            .await?;
        dioxus::Ok(())
    });

    let mut continue_transform_stream = use_signal_sync(|| false);
    let mut start_transform_stream = use_action(move || async move {
        continue_transform_stream.set(true);
        transform_responses.clear();
        let stream = transform_stream(Streaming::new(
            futures::stream::unfold(0, move |index| async move {
                if !continue_transform_stream() {
                    return None;
                }
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                let dog = Dog {
                    name: format!("Dog {}", index),
                    age: (index % 10) as u8,
                };
                Some((dog, index + 1))
            }),
        ))
        .await?;
        stream
            .into_inner()
            .try_for_each(move |text| async move {
                transform_responses.push(text);
                Ok(())
            })
            .await?;
        dioxus::Ok(())
    });

    rsx! {
        div {
            button { onclick: move |_| start_text_stream.call(), "Start text stream" }
            button { onclick: move |_| start_text_stream.cancel(), "Stop text stream" }
            pre { "{text_responses}" }
        }
        div {
            button { onclick: move |_| start_json_stream.call(), "Start JSON stream" }
            button { onclick: move |_| start_json_stream.cancel(), "Stop JSON stream" }
            for dog in json_responses.read().iter() {
                pre { "{dog:?}" }
            }
        }
        div {
            button { onclick: move |_| start_echo_stream.call(), "Start echo stream" }
            button { onclick: move |_| continue_echo_stream.set(false), "Stop echo stream" }
            for dog in echo_responses.read().iter() {
                pre { "{dog:?}" }
            }
        }
        div {
            button { onclick: move |_| start_transform_stream.call(), "Start transform stream" }
            button { onclick: move |_| continue_transform_stream.set(false), "Stop transform stream" }
            for text in transform_responses.read().iter() {
                pre { "{text}" }
            }
        }
    }
}

/// The `TextStream` type is an alias for `Streaming<String>` with a text/plain encoding.
///
/// The `TextStream::new()` method takes anything that implements `Stream<Item = String>`, so
/// we can use a channel to send strings from a background task.
#[get("/api/test_stream?start")]
async fn text_stream(start: Option<i32>) -> Result<TextStream> {
    let (tx, rx) = futures::channel::mpsc::unbounded();

    tokio::spawn(async move {
        let mut count = start.unwrap_or(0);
        loop {
            let message = format!("Hello, world! {}", count);
            if tx.unbounded_send(message).is_err() {
                break;
            }

            count += 1;
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }
    });

    Ok(Streaming::new(rx))
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct Dog {
    name: String,
    age: u8,
}

/// A custom `Streaming<T, E>` endpoint that streams JSON-encoded `Dog` structs to the client.
///
/// Dioxus provides the `JsonEncoding` type which can be used to encode and decode JSON data.
#[get("/api/json_stream")]
async fn json_stream() -> Result<Streaming<Dog, JsonEncoding>> {
    let (tx, rx) = futures::channel::mpsc::unbounded();

    tokio::spawn(async move {
        let mut count = 0;
        loop {
            let dog = Dog {
                name: format!("Dog {}", count),
                age: (count % 10) as u8,
            };
            if tx.unbounded_send(dog).is_err() {
                // If the channel is closed, stop sending chunks
                break;
            }
            count += 1;
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }
    });

    Ok(Streaming::new(rx))
}

/// An example of streaming raw bytes to the client using `Streaming<Bytes>`.
/// This is useful for sending binary data, such as images, files, or zero-copy data.
#[get("/api/byte_stream")]
async fn byte_stream() -> Result<Streaming<Bytes>> {
    let (tx, rx) = futures::channel::mpsc::unbounded();

    tokio::spawn(async move {
        let mut count = 0;
        loop {
            let bytes = vec![count; 10];
            if tx.unbounded_send(bytes.into()).is_err() {
                break;
            }
            count = (count + 1) % 255;
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }
    });

    Ok(Streaming::new(rx))
}

/// An example of echoing the stream back to the client.
#[post("/api/echo_stream")]
async fn echo_stream(stream: Streaming<Dog, JsonEncoding>) -> Result<Streaming<Dog, JsonEncoding>> {
    Ok(stream)
}

/// An example of transforming the stream on the server.
#[post("/api/transform_stream")]
async fn transform_stream(stream: Streaming<Dog, JsonEncoding>) -> Result<TextStream> {
    Ok(Streaming::new(stream.into_inner().filter_map(
        |dog| async {
            dog.ok()
                .map(|dog| format!("name: {}, age: {}", dog.name, dog.age))
        },
    )))
}
