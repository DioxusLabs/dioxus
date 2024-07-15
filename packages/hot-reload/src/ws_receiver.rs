use crate::DevserverMsg;
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio_tungstenite::{
    tungstenite::{Message, Result as TtResult},
    MaybeTlsStream, WebSocketStream,
};

pub fn connect(mut callback: impl FnMut(DevserverMsg) + Send + 'static) {
    tokio::spawn(async move {
        let Some(Ok(mut recv)) = NativeReceiver::create_from_cli().await else {
            return;
        };
        while let Some(msg) = recv.next().await {
            match msg {
                Ok(msg) => callback(msg),
                Err(_e) => {}
            }
        }
    });
}

/// A receiver for messages from the devserver
///
/// Calling `next` will watch the channel for the next valid message from the devserver
pub struct NativeReceiver {
    socket: WebSocketStream<MaybeTlsStream<TcpStream>>,
}

impl NativeReceiver {
    /// Connect to the devserver
    async fn create(url: String) -> TtResult<Self> {
        let (socket, _ws) = tokio_tungstenite::connect_async(&url).await?;
        Ok(Self { socket })
    }

    /// Connect to the devserver with an address from the CLI. Returns None if the current application was not run with the CLI
    pub async fn create_from_cli() -> Option<TtResult<Self>> {
        let cli_args = dioxus_cli_config::RuntimeCLIArguments::from_cli()?;
        let addr = cli_args.cli_address();
        Some(Self::create(format!("ws://{addr}/_dioxus")).await)
    }

    /// Wait for the next message from the devserver
    ///
    /// Returns None when the connection is closed or socket.next() returns None
    pub async fn next(&mut self) -> Option<TtResult<DevserverMsg>> {
        loop {
            let res = self.socket.next().await?;

            match res {
                Ok(res) => match res {
                    Message::Text(text) => {
                        let leaked: &'static str = Box::leak(text.into_boxed_str());
                        let msg = serde_json::from_str::<DevserverMsg>(leaked);
                        if let Ok(msg) = msg {
                            return Some(Ok(msg));
                        }
                    }
                    // send a pong
                    Message::Ping(_) => {
                        let _ = self.socket.send(Message::Pong(vec![])).await;
                    }
                    Message::Close(_) => return None,
                    Message::Binary(_) => {}
                    Message::Pong(_) => {}
                    Message::Frame(_) => {}
                },
                Err(e) => return Some(Err(e)),
            };
        }
    }
}
