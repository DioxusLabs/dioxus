use dioxus_core::VirtualDom;
use interprocess::local_socket::LocalSocketStream;
// use interprocess::local_socket::{LocalSocketListener, LocalSocketStream};
use std::io::{BufRead, BufReader};
use std::time::Duration;
use std::{sync::Arc, sync::Mutex};

fn _handle_error(connection: std::io::Result<LocalSocketStream>) -> Option<LocalSocketStream> {
    connection
        .map_err(|error| eprintln!("Incoming connection failed: {}", error))
        .ok()
}

pub(crate) fn _init(_dom: &VirtualDom) {
    let latest_in_connection: Arc<Mutex<Option<BufReader<LocalSocketStream>>>> =
        Arc::new(Mutex::new(None));
    let _latest_in_connection_handle = latest_in_connection.clone();

    // connect to processes for incoming data
    std::thread::spawn(move || {
        // if let Ok(listener) = LocalSocketListener::bind("@dioxusin") {
        //     for conn in listener.incoming().filter_map(handle_error) {
        //         *latest_in_connection_handle.lock().unwrap() = Some(BufReader::new(conn));
        //     }
        // }
    });

    std::thread::spawn(move || {
        loop {
            if let Some(conn) = &mut *latest_in_connection.lock().unwrap() {
                let mut buf = String::new();
                match conn.read_line(&mut buf) {
                    Ok(_) => {
                        todo!()
                        // let msg: SetTemplateMsg = serde_json::from_str(&buf).unwrap();
                        // channel
                        //     .start_send(SchedulerMsg::SetTemplate(Box::new(msg)))
                        //     .unwrap();
                    }
                    Err(err) => {
                        if err.kind() != std::io::ErrorKind::WouldBlock {
                            break;
                        }
                    }
                }
            }
            // give the error handler time to take the mutex
            std::thread::sleep(Duration::from_millis(100));
        }
    });
}
