// #![deny(warnings)]
use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use futures_util::{pin_mut, SinkExt, StreamExt, TryFutureExt};
use tokio::sync::{mpsc, RwLock};
use tokio_stream::wrappers::UnboundedReceiverStream;
use warp::ws::{Message, WebSocket};
use warp::Filter;

mod events;

/// Our global unique user id counter.
static NEXT_USER_ID: AtomicUsize = AtomicUsize::new(1);

/// Our state of currently connected users.
///
/// - Key is their id
/// - Value is a sender of `warp::ws::Message`
type Users = Arc<RwLock<HashMap<usize, mpsc::UnboundedSender<Message>>>>;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let state = Users::default();

    let chat = warp::path("chat")
        .and(warp::ws())
        .and(warp::any().map(move || state.clone()))
        .map(|ws: warp::ws::Ws, users| ws.on_upgrade(move |socket| user_connected(socket, users)));

    let index = warp::path::end().map(|| warp::reply::html(INDEX_HTML));

    let routes = index.or(chat);

    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}

async fn user_connected(ws: WebSocket, users: Users) {
    // Use a counter to assign a new unique ID for this user.
    let my_id = NEXT_USER_ID.fetch_add(1, Ordering::Relaxed);

    eprintln!("new chat user: {}", my_id);

    // Split the socket into a sender and receive of messages.
    let (mut user_ws_tx, mut user_ws_rx) = ws.split();

    let (event_tx, event_rx) = mpsc::unbounded_channel();
    let (edits_tx, edits_rx) = mpsc::unbounded_channel();

    let mut edits_rx = UnboundedReceiverStream::new(edits_rx);
    let mut event_rx = UnboundedReceiverStream::new(event_rx);

    tokio::task::spawn_blocking(move || {
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async move {
                use dioxus::prelude::*;

                fn app(cx: Scope) -> Element {
                    let (count, set_count) = use_state(&cx, || 0);
                    cx.render(rsx! {
                        div { "hello world: {count}" }
                        button {
                            onclick: move |_| set_count(count + 1),
                            "increment"
                        }
                    })
                }

                let mut vdom = VirtualDom::new(app);

                let edits = vdom.rebuild();

                let serialized = serde_json::to_string(&edits.edits).unwrap();
                edits_tx.send(serialized).unwrap();

                loop {
                    use futures_util::future::{select, Either};

                    let new_event = {
                        let vdom_fut = vdom.wait_for_work();

                        pin_mut!(vdom_fut);

                        match select(event_rx.next(), vdom_fut).await {
                            Either::Left((l, _)) => l,
                            Either::Right((_, _)) => None,
                        }
                    };

                    if let Some(new_event) = new_event {
                        vdom.handle_message(dioxus::core::SchedulerMsg::Event(new_event));
                    } else {
                        let mutations = vdom.work_with_deadline(|| false);
                        for mutation in mutations {
                            let edits = serde_json::to_string(&mutation.edits).unwrap();
                            edits_tx.send(edits).unwrap();
                        }
                    }
                }
            })
    });

    loop {
        use futures_util::future::{select, Either};

        match select(user_ws_rx.next(), edits_rx.next()).await {
            Either::Left((l, _)) => {
                if let Some(Ok(msg)) = l {
                    if let Ok(Some(msg)) = msg.to_str().map(events::parse_ipc_message) {
                        let user_event = events::trigger_from_serialized(msg.params);
                        event_tx.send(user_event).unwrap();
                    }
                }
            }
            Either::Right((edits, _)) => {
                if let Some(edits) = edits {
                    // send the edits to the client
                    if user_ws_tx.send(Message::text(edits)).await.is_err() {
                        break;
                    }
                }
            }
        }
    }

    // log::info!("");
}

async fn user_message(my_id: usize, msg: Message, users: &Users) {
    // Skip any non-Text messages...
    let msg = if let Ok(s) = msg.to_str() {
        s
    } else {
        return;
    };

    let new_msg = format!("<User#{}>: {}", my_id, msg);

    // New message from this user, send it to everyone else (except same uid)...
    for (&uid, tx) in users.read().await.iter() {
        if my_id != uid {
            if let Err(_disconnected) = tx.send(Message::text(new_msg.clone())) {
                // The tx is disconnected, our `user_disconnected` code
                // should be happening in another task, nothing more to
                // do here.
            }
        }
    }
}

async fn user_disconnected(my_id: usize, users: &Users) {
    eprintln!("good bye user: {}", my_id);

    // Stream closed up, so remove from the user list
    users.write().await.remove(&my_id);
}

static INDEX_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
    <head>
        <title>Warp Chat</title>
    </head>
    <body>
        <h1>Warp chat</h1>
        <div id="chat">
            <p><em>Connecting...</em></p>
        </div>
        <input type="text" id="text" />
        <button type="button" id="send">Send</button>
        <script type="text/javascript">
        const chat = document.getElementById('chat');
        const text = document.getElementById('text');
        const uri = 'ws://' + location.host + '/chat';
        const ws = new WebSocket(uri);

        function message(data) {
            const line = document.createElement('p');
            line.innerText = data;
            chat.appendChild(line);
        }

        ws.onopen = function() {
            chat.innerHTML = '<p><em>Connected!</em></p>';
        };

        ws.onmessage = function(msg) {
            message(msg.data);
        };

        ws.onclose = function() {
            chat.getElementsByTagName('em')[0].innerText = 'Disconnected!';
        };

        send.onclick = function() {
            const msg = text.value;
            ws.send(msg);
            text.value = '';

            message('<You>: ' + msg);
        };
        </script>
    </body>
</html>
"#;
