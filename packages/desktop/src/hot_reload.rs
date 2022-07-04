use dioxus_core::VirtualDom;
use dioxus_rsx_interpreter::{error::Error, ErrorHandler, SetManyRsxMessage, RSX_CONTEXT};
use interprocess::local_socket::{LocalSocketListener, LocalSocketStream};
use std::io::{BufRead, BufReader, Write};
use std::time::Duration;
use std::{sync::Arc, sync::Mutex};

fn handle_error(connection: std::io::Result<LocalSocketStream>) -> Option<LocalSocketStream> {
    connection
        .map_err(|error| eprintln!("Incoming connection failed: {}", error))
        .ok()
}

pub(crate) fn init(dom: &VirtualDom) {
    let latest_in_connection: Arc<Mutex<Option<BufReader<LocalSocketStream>>>> =
        Arc::new(Mutex::new(None));
    let latest_in_connection_handle = latest_in_connection.clone();
    let latest_out_connection: Arc<Mutex<Option<BufReader<LocalSocketStream>>>> =
        Arc::new(Mutex::new(None));
    let latest_out_connection_handle = latest_out_connection.clone();

    struct DesktopErrorHandler {
        latest_connection: Arc<Mutex<Option<BufReader<LocalSocketStream>>>>,
    }
    impl ErrorHandler for DesktopErrorHandler {
        fn handle_error(&self, err: Error) {
            if let Some(conn) = &mut *self.latest_connection.lock().unwrap() {
                conn.get_mut()
                    .write_all((serde_json::to_string(&err).unwrap() + "\n").as_bytes())
                    .unwrap();
            } else {
                panic!("{}", err);
            }
        }
    }

    RSX_CONTEXT.set_error_handler(DesktopErrorHandler {
        latest_connection: latest_out_connection_handle,
    });
    RSX_CONTEXT.provide_scheduler_channel(dom.get_scheduler_channel());

    // connect to processes for incoming data
    std::thread::spawn(move || {
        if let Ok(listener) = LocalSocketListener::bind("@dioxusin") {
            for conn in listener.incoming().filter_map(handle_error) {
                *latest_in_connection_handle.lock().unwrap() = Some(BufReader::new(conn));
            }
        }
    });

    // connect to processes for outgoing errors
    std::thread::spawn(move || {
        if let Ok(listener) = LocalSocketListener::bind("@dioxusout") {
            for conn in listener.incoming().filter_map(handle_error) {
                *latest_out_connection.lock().unwrap() = Some(BufReader::new(conn));
            }
        }
    });

    std::thread::spawn(move || {
        loop {
            if let Some(conn) = &mut *latest_in_connection.lock().unwrap() {
                let mut buf = String::new();
                match conn.read_line(&mut buf) {
                    Ok(_) => {
                        let msgs: SetManyRsxMessage = serde_json::from_str(&buf).unwrap();
                        RSX_CONTEXT.extend(msgs);
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
