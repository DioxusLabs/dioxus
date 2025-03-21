use dioxus::prelude::dioxus_devtools;
use subsecond::JumpTable;

#[no_mangle]
pub extern "C" fn aslr_reference() -> u64 {
    aslr_reference as *const () as u64
}

pub fn initialize() {
    if let Some(endpoint) = dioxus::cli_config::devserver_ws_endpoint() {
        // dioxus_devtools::connect(endpoint, |msg| match msg {
        //     dioxus_devtools::DevserverMsg::HotReload(hot_reload_msg) => {
        //         if let Some(jump_table) = hot_reload_msg.jump_table {
        //             unsafe { subsecond::run_patch(jump_table) };
        //         }
        //     }
        //     _ => {}
        // });

        // don't boot the default
        return;
    }

    // Spawn a thread that will read bytes from the fd
    // the host process will write new bytes to the fd when it wants to reload the binary
    std::thread::spawn(|| {
        let endpoint =
            std::env::var("HOTRELOAD_ENDPOINT").unwrap_or("ws://localhost:9393".to_string());

        let (mut websocket, _req) = match tungstenite::connect(endpoint.clone()) {
            Ok((websocket, req)) => (websocket, req),
            Err(_) => panic!("Failed to connect to hotreload endpoint"),
        };

        websocket
            .send(tungstenite::Message::Text(aslr_reference().to_string()))
            .unwrap();

        while let Ok(msg) = websocket.read() {
            if let tungstenite::Message::Binary(bytes) = msg {
                if let Ok(msg) = bincode::deserialize::<JumpTable>(bytes.as_ref()) {
                    unsafe { subsecond::run_patch(msg) };
                }
            }
        }
    });
}
