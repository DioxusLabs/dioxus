use crate::DevserverMsg;
use std::io::Read;

pub fn connect(mut callback: impl FnMut(DevserverMsg) + Send + 'static) {
    // Hi!
    //
    // yes, we read-raw from a tcp socket
    // don't think about it too much :)
    //
    // we just don't want to bring in tls + tokio for just hotreloading
    std::thread::spawn(move || {
        let connect = std::net::TcpStream::connect("127.0.0.1:8080");
        let Ok(mut stream) = connect else {
            return;
        };

        loop {}

        // let mut buf = [0; 1024];
        // loop {
        //     let len = stream.read(&mut buf).unwrap();
        //     if len == 0 {
        //         break;
        //     }
        //     let msg = String::from_utf8_lossy(&buf[..len]);
        //     callback(serde_json::from_str(&msg).unwrap());
        // }
    });
}

/// A receiver for messages from the devserver
///
/// Calling `next` will watch the channel for the next valid message from the devserver
pub struct NativeReceiver {
    // socket: WebSocketStream<MaybeTlsStream<TcpStream>>,
}

impl NativeReceiver {
    // /// Connect to the devserver
    // async fn create(url: String) -> TtResult<Self> {
    //     let (socket, _ws) = tokio_tungstenite::connect_async(&url).await.unwrap();
    //     Ok(Self { socket })
    // }

    // /// Connect to the devserver with an address from the CLI. Returns None if the current application was not run with the CLI
    // pub async fn create_from_cli() -> Option<TtResult<Self>> {
    //     // todo: allow external configuration of this address for use by mobile when launching
    //     //       from the ios-deploy tooling. This could be stored in a config file= that gets
    //     //       uploaded to the device.
    //     let addr =
    //         dioxus_cli_config::RuntimeCLIArguments::from_cli().map(|args| args.cli_address())?;
    //     Some(Self::create(format!("ws://{addr}/_dioxus")).await)
    // }

    // /// Wait for the next message from the devserver
    // ///
    // /// Returns None when the connection is closed or socket.next() returns None
    // pub async fn next(&mut self) -> Option<TtResult<DevserverMsg>> {
    //     loop {
    //         let res = self.socket.next().await?;

    //         match res {
    //             Ok(res) => match res {
    //                 Message::Text(text) => {
    //                     // let leaked: &'static str = Box::leak(text.into_boxed_str());
    //                     let msg = serde_json::from_str::<DevserverMsg>(&text);
    //                     if let Ok(msg) = msg {
    //                         return Some(Ok(msg));
    //                     }
    //                 }
    //                 // send a pong
    //                 Message::Ping(_) => {
    //                     let _ = self.socket.send(Message::Pong(vec![])).await;
    //                 }
    //                 Message::Close(_) => return None,
    //                 Message::Binary(_) => {}
    //                 Message::Pong(_) => {}
    //                 Message::Frame(_) => {}
    //             },
    //             Err(e) => return Some(Err(e)),
    //         };
    //     }
    // }
}
