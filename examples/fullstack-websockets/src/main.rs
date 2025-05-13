#![allow(non_snake_case)]
use dioxus::prelude::{
    server_fn::{codec::JsonEncoding, BoxedStream, Websocket},
    *,
};
use futures::{channel::mpsc, SinkExt, StreamExt};

fn main() {
    launch(app);
}

fn app() -> Element {
    let mut uppercase = use_signal(String::new);
    let mut uppercase_channel = use_signal(|| None);

    // Start the websocket connection in a background task
    use_future(move || async move {
        let (tx, rx) = mpsc::channel(1);
        let mut receiver = uppercase_ws(rx.into()).await.unwrap();
        // Store the channel in a signal for use in the input handler
        uppercase_channel.set(Some(tx));
        // Whenever we get a message from the server, update the uppercase signal
        while let Some(Ok(msg)) = receiver.next().await {
            uppercase.set(msg);
        }
    });

    rsx! {
        input {
            oninput: move |e| async move {
                if let Some(mut uppercase_channel) = uppercase_channel() {
                    let msg = e.value();
                    uppercase_channel.send(Ok(msg)).await.unwrap();
                }
            },
        }
        "Uppercase: {uppercase}"
    }
}

// The server macro accepts a protocol parameter which implements the protocol trait. The protocol
// controls how the inputs and outputs are encoded when handling the server function. In this case,
// the websocket<json, json> protocol can encode a stream input and stream output where messages are
// serialized as JSON
#[server(protocol = Websocket<JsonEncoding, JsonEncoding>)]
async fn uppercase_ws(
    input: BoxedStream<String, ServerFnError>,
) -> Result<BoxedStream<String, ServerFnError>, ServerFnError> {
    let mut input = input;

    // Create a channel with the output of the websocket
    let (mut tx, rx) = mpsc::channel(1);

    // Spawn a task that processes the input stream and sends any new messages to the output
    tokio::spawn(async move {
        while let Some(msg) = input.next().await {
            if tx
                .send(msg.map(|msg| msg.to_ascii_uppercase()))
                .await
                .is_err()
            {
                break;
            }
        }
    });

    // Return the output stream
    Ok(rx.into())
}
