use std::{
    io::{BufRead, BufReader, Write},
    path::PathBuf,
    str::FromStr,
    sync::{Arc, Mutex},
};

use dioxus_core::Template;
use dioxus_rsx::{
    hot_reload::{FileMap, UpdateResult},
    HotReloadingContext,
};
use interprocess::local_socket::{LocalSocketListener, LocalSocketStream};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};

#[cfg(debug_assertions)]
pub use dioxus_html::HtmlCtx;

/// Initialize the hot reloading listener on the given path
pub fn init<Ctx: HotReloadingContext + Send + 'static>(
    root_path: &'static str,
    listening_paths: &'static [&'static str],
    log: bool,
) {
    if let Ok(crate_dir) = PathBuf::from_str(root_path) {
        let temp_file = std::env::temp_dir().join("@dioxusin");
        let channels = Arc::new(Mutex::new(Vec::new()));
        let file_map = Arc::new(Mutex::new(FileMap::<Ctx>::new(crate_dir.clone())));
        if let Ok(local_socket_stream) = LocalSocketListener::bind(temp_file.as_path()) {
            // listen for connections
            std::thread::spawn({
                let file_map = file_map.clone();
                let channels = channels.clone();
                move || {
                    for connection in local_socket_stream.incoming() {
                        if let Ok(mut connection) = connection {
                            // send any templates than have changed before the socket connected
                            let templates: Vec<_> = {
                                file_map
                                    .lock()
                                    .unwrap()
                                    .map
                                    .values()
                                    .filter_map(|(_, template_slot)| *template_slot)
                                    .collect()
                            };
                            for template in templates {
                                if !send_template(template, &mut connection) {
                                    continue;
                                }
                            }
                            channels.lock().unwrap().push(connection);
                            if log {
                                println!("Connected to hot reloading 🚀");
                            }
                        }
                    }
                }
            });

            // watch for changes
            std::thread::spawn(move || {
                let mut last_update_time = chrono::Local::now().timestamp();

                let (tx, rx) = std::sync::mpsc::channel();

                let mut watcher = RecommendedWatcher::new(tx, notify::Config::default()).unwrap();

                for path in listening_paths {
                    match PathBuf::from_str(path) {
                        Ok(path) => {
                            let full_path = crate_dir.join(path);
                            if let Err(err) = watcher.watch(&full_path, RecursiveMode::Recursive) {
                                if log {
                                    println!(
                                        "hot reloading failed to start watching {full_path:?}:\n{err:?}",
                                    );
                                }
                            }
                        }
                        Err(err) => {
                            if log {
                                println!("hot reloading failed to create path:\n{:?}", err);
                            }
                        }
                    }
                }

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
                                match file_map
                                    .lock()
                                    .unwrap()
                                    .update_rsx(&path, crate_dir.as_path())
                                {
                                    UpdateResult::UpdatedRsx(msgs) => {
                                        for msg in msgs {
                                            let mut i = 0;
                                            while i < channels.len() {
                                                let channel = &mut channels[i];
                                                if send_template(msg, channel) {
                                                    i += 1;
                                                } else {
                                                    channels.remove(i);
                                                }
                                            }
                                        }
                                    }
                                    UpdateResult::NeedsRebuild => {
                                        if log {
                                            println!(
                                                "Rebuild needed... shutting down hot reloading"
                                            );
                                        }
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

fn send_template(template: Template<'static>, channel: &mut impl Write) -> bool {
    if let Ok(msg) = serde_json::to_string(&template) {
        if channel.write_all(msg.as_bytes()).is_err() {
            return false;
        }
        if channel.write_all(&[b'\n']).is_err() {
            return false;
        }
        true
    } else {
        false
    }
}

/// Connect to the hot reloading listener. The callback provided will be called every time a template change is detected
pub fn connect(mut f: impl FnMut(Template<'static>) + Send + 'static) {
    std::thread::spawn(move || {
        let temp_file = std::env::temp_dir().join("@dioxusin");
        if let Ok(socket) = LocalSocketStream::connect(temp_file.as_path()) {
            let mut buf_reader = BufReader::new(socket);
            loop {
                let mut buf = String::new();
                match buf_reader.read_line(&mut buf) {
                    Ok(_) => {
                        let template: Template<'static> =
                            serde_json::from_str(Box::leak(buf.into_boxed_str())).unwrap();
                        f(template);
                    }
                    Err(err) => {
                        if err.kind() != std::io::ErrorKind::WouldBlock {
                            break;
                        }
                    }
                }
            }
        }
    });
}

/// Start the hot reloading server
///
/// Pass any number of paths to listen for changes on relative to the crate root as strings.
/// If no paths are passed, it will listen on the src and examples folders.
#[macro_export]
macro_rules! hot_reload_init {
    ($(@ $ctx:ident)? $($t: ident)*) => {
        #[cfg(debug_assertions)]
        dioxus_hot_reload::init::<hot_reload_init!(@ctx: $($ctx)?)>(core::env!("CARGO_MANIFEST_DIR"), &["src", "examples"], hot_reload_init!(@log: $($t)*))
    };

    ($(@ $ctx:ident)? $($paths: literal),* $(,)? $($t: ident)*) => {
        #[cfg(debug_assertions)]
        dioxus_hot_reload::init::<hot_reload_init!(@ctx: $($ctx)?)>(core::env!("CARGO_MANIFEST_DIR"), &[$($paths),*], hot_reload_init!(@log: $($t)*))
    };

    (@log:) => {
        false
    };

    (@log: enable logging) => {
        true
    };

    (@log: disable logging) => {
        false
    };

    (@ctx: $ctx: ident) => {
        $ctx
    };

    (@ctx: ) => {
        dioxus_hot_reload::HtmlCtx
    };
}
