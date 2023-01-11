use std::{
    io::Write,
    path::PathBuf,
    str::FromStr,
    sync::{Arc, Mutex},
};

pub use dioxus_hot_reload_macro::hot_reload;
use dioxus_html::HtmlCtx;
use dioxus_rsx::hot_reload::{FileMap, UpdateResult};
use interprocess::local_socket::LocalSocketListener;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};

pub fn init(path: &'static str) {
    if let Ok(crate_dir) = PathBuf::from_str(path) {
        let temp_file = std::env::temp_dir().join("@dioxusin");
        let channels = Arc::new(Mutex::new(Vec::new()));
        if let Ok(local_socket_stream) = LocalSocketListener::bind(temp_file.as_path()) {
            // listen for connections
            std::thread::spawn({
                let channels = channels.clone();
                move || {
                    for connection in local_socket_stream.incoming() {
                        if let Ok(connection) = connection {
                            channels.lock().unwrap().push(connection);
                            println!("Connected to hot reloading 🚀");
                        }
                    }
                }
            });

            // watch for changes
            std::thread::spawn(move || {
                let mut last_update_time = chrono::Local::now().timestamp();
                let mut file_map = FileMap::<HtmlCtx>::new(crate_dir.clone());

                let (tx, rx) = std::sync::mpsc::channel();

                let mut watcher = RecommendedWatcher::new(tx, notify::Config::default()).unwrap();

                let mut examples_path = crate_dir.clone();
                examples_path.push("examples");
                let _ = watcher.watch(&examples_path, RecursiveMode::Recursive);
                let mut src_path = crate_dir.clone();
                src_path.push("src");
                let _ = watcher.watch(&src_path, RecursiveMode::Recursive);

                for evt in rx {
                    // Give time for the change to take effect before reading the file
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    if chrono::Local::now().timestamp() > last_update_time {
                        if let Ok(evt) = evt {
                            let mut channels = channels.lock().unwrap();
                            for path in &evt.paths {
                                // skip non rust files
                                if path.extension().and_then(|p| p.to_str()) != Some("rs") {
                                    continue;
                                }

                                // find changes to the rsx in the file
                                match file_map.update_rsx(&path, crate_dir.as_path()) {
                                    UpdateResult::UpdatedRsx(msgs) => {
                                        for msg in msgs {
                                            for channel in channels.iter_mut() {
                                                let msg = serde_json::to_string(&msg).unwrap();
                                                channel.write_all(msg.as_bytes()).unwrap();
                                                channel.write_all(&[b'\n']).unwrap();
                                            }
                                        }
                                    }
                                    UpdateResult::NeedsRebuild => {
                                        println!("Rebuild needed... shutting down hot reloading");
                                        return;
                                    }
                                }
                            }
                        }
                        last_update_time = chrono::Local::now().timestamp();
                    }
                }
            });
        }
    }
}
