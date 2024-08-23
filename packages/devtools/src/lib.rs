use dioxus_core::{ScopeId, VirtualDom};
pub use dioxus_devtools_types::*;
use dioxus_signals::Writable;
use warnings::Warning;

/// Applies template and literal changes to the VirtualDom
///
/// Assets need to be handled by the renderer.
pub fn apply_changes(dom: &VirtualDom, msg: &HotReloadMsg) {
    dom.runtime().on_scope(ScopeId::ROOT, || {
        let ctx = dioxus_signals::get_global_context();

        for template in &msg.templates {
            let id = &template.location;
            let value = template.template.clone();
            if let Some(mut signal) = ctx.get_signal_with_key(id) {
                dioxus_signals::warnings::signal_read_and_write_in_reactive_scope::allow(|| {
                    dioxus_signals::warnings::signal_write_in_component_body::allow(|| {
                        signal.set(Some(value));
                    });
                });
            }
        }
    });
}

/// Connect to the devserver and handle its messages with a callback.
///
/// This doesn't use any form of security or protocol, so it's not safe to expose to the internet.
#[cfg(not(target_arch = "wasm32"))]
pub fn connect(addr: String, mut callback: impl FnMut(DevserverMsg) + Send + 'static) {
    use std::{
        io::{self, BufRead},
        net::{SocketAddr, TcpListener, ToSocketAddrs},
    };

    std::thread::spawn(move || {
        let (mut websocket, req) = match tungstenite::connect(addr.clone()) {
            Ok((websocket, req)) => (websocket, req),
            Err(err) => {
                eprintln!("Failed to connect to devserver at {} because {}", addr, err);
                return;
            }
        };

        println!("Connected to devserver at {}", addr);

        while let Ok(msg) = websocket.read() {
            match msg {
                tungstenite::Message::Text(text) => {
                    if let Ok(msg) = serde_json::from_str(&text) {
                        callback(msg);
                    } else {
                        eprintln!("Failed to parse message from devserver: {:?}", text);
                    }
                }
                msg => {
                    println!("Received a non-text message: {:?}", msg);
                }
            }
        }

        eprintln!("Disconnected from devserver");

        // let connect = std::net::TcpStream::connect(addr);
        // let Ok(mut stream) = connect else {
        //     return;
        // };

        // // Wrap the stream in a BufReader, so we can use the BufRead methods
        // let mut reader = io::BufReader::new(&mut stream);

        // // Loop and read lines from the stream
        // loop {
        //     let mut buf = String::new();
        //     let msg = reader.read_line(&mut buf);

        //     let Ok(amt) = msg else {
        //         break;
        //     };

        //     // eof received - connection closed
        //     if amt == 0 {
        //         break;
        //     }

        //     reader.consume(amt);

        //     if let Ok(msg) = serde_json::from_str(&buf) {
        //         callback(msg);
        //     } else {
        //         eprintln!("Failed to parse message from devserver: {:?}", buf);
        //     }
        // }
    });
}
