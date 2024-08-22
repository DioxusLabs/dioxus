use dioxus_devtools::*;
use std::io::{BufWriter, Write};

#[test]
fn roundtrip() {
    println!("Starting server");

    let callback = |msg: DevserverMsg| {
        println!("msg received! {msg:?}");
    };

    let bind = std::net::TcpListener::bind("127.0.0.1:8080").expect("Failed to bind port");

    connect("127.0.0.1:8080".parse().unwrap(), callback);

    let (stream, _addr) = bind.accept().unwrap();
    {
        println!("Accepted connection");

        let mut writer = BufWriter::new(stream);

        for _x in 0..3 {
            let line = serde_json::to_string(&DevserverMsg::HotReload(HotReloadMsg {
                templates: vec![],
                assets: vec!["/asd/bcc".into()],
                unknown_files: vec![],
            }))
            .unwrap();
            writer.write(line.as_bytes()).unwrap();
            writer.write("\n".as_bytes()).unwrap();
            writer.flush().unwrap();
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }

    std::thread::sleep(std::time::Duration::from_secs(1));
}
