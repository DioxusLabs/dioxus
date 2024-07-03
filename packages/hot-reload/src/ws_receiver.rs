use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use tokio::net::TcpStream;
use tokio_tungstenite::{tungstenite::Message, MaybeTlsStream, WebSocketStream};

use crate::DevserverMsg;

pub struct NativeReceiver {
    socket: WebSocketStream<MaybeTlsStream<TcpStream>>,
}

impl NativeReceiver {
    pub async fn connect(uri: String) -> Self {
        let (socket, ws) = tokio_tungstenite::connect_async(&uri).await.unwrap();

        Self { socket }
    }

    pub async fn next(
        &mut self,
    ) -> Option<Result<DevserverMsg, tokio_tungstenite::tungstenite::Error>> {
        let res = self.socket.next().await;

        res.map(|f| {
            f.map(|f| {
                println!("Received message: {:?}", f);

                match f {
                    Message::Text(_) => todo!(),
                    Message::Binary(_) => todo!(),
                    Message::Ping(_) => todo!(),
                    Message::Pong(_) => todo!(),
                    Message::Close(_) => todo!(),
                    Message::Frame(_) => todo!(),
                }

                f
            })
        });
    }
}
