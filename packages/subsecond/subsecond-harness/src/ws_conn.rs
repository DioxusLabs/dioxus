use subsecond::JumpTable;

pub fn initialize() {
    // dx already has subsecond integrated, don't boot it twice
    if dioxus::cli_config::devserver_ws_endpoint().is_some() {
        return;
    }

    // Spawn a thread that will read bytes from the fd
    // the host process will write newa bytes to the fd when it wants to reload the binary
    #[cfg(not(target_arch = "wasm32"))]
    std::thread::spawn(|| {
        let endpoint =
            std::env::var("HOTRELOAD_ENDPOINT").unwrap_or("ws://localhost:9393".to_string());

        let (mut websocket, _req) = match tungstenite::connect(endpoint.clone()) {
            Ok((websocket, req)) => (websocket, req),
            Err(_) => panic!("Failed to connect to hotreload endpoint"),
        };

        websocket
            .send(tungstenite::Message::Text(
                subsecond::aslr_reference().to_string(),
            ))
            .unwrap();

        while let Ok(msg) = websocket.read() {
            if let tungstenite::Message::Binary(bytes) = msg {
                if let Ok(msg) = bincode::deserialize::<JumpTable>(bytes.as_ref()) {
                    unsafe { subsecond::apply_patch(msg) };
                }
            }
        }
    });
}
