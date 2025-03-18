use subsecond::JumpTable;

pub fn initialize() {
    // Spawn a thread that will read bytes from the fd
    // the host process will write new bytes to the fd when it wants to reload the binary
    std::thread::spawn(|| {
        let endpoint =
            std::env::var("HOTRELOAD_ENDPOINT").unwrap_or("ws://localhost:9393".to_string());

        let (mut websocket, _req) = match tungstenite::connect(endpoint.clone()) {
            Ok((websocket, req)) => (websocket, req),
            Err(_) => panic!("Failed to connect to hotreload endpoint"),
        };

        while let Ok(msg) = websocket.read() {
            if let tungstenite::Message::Binary(bytes) = msg {
                if let Ok(msg) = bincode::deserialize::<JumpTable>(bytes.as_ref()) {
                    subsecond::run_patch(msg);
                }
            }
        }
    });
}
