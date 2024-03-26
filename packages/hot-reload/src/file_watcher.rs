use std::{
    io::Write,
    path::PathBuf,
    str::FromStr,
    sync::{Arc, Mutex},
};

use crate::HotReloadMsg;
use dioxus_rsx::{
    hot_reload::{FileMap, FileMapBuildResult, UpdateResult},
    HotReloadingContext,
};
use interprocess::local_socket::LocalSocketListener;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};

#[cfg(feature = "file_watcher")]
use dioxus_html::HtmlCtx;

pub struct Config<Ctx: HotReloadingContext> {
    root_path: &'static str,
    listening_paths: &'static [&'static str],
    excluded_paths: &'static [&'static str],
    log: bool,
    rebuild_with: Option<Box<dyn FnMut() -> bool + Send + 'static>>,
    phantom: std::marker::PhantomData<Ctx>,
}

impl<Ctx: HotReloadingContext> Default for Config<Ctx> {
    fn default() -> Self {
        Self {
            root_path: "",
            listening_paths: &[""],
            excluded_paths: &["./target"],
            log: true,
            rebuild_with: None,
            phantom: std::marker::PhantomData,
        }
    }
}

#[cfg(feature = "file_watcher")]
impl Config<HtmlCtx> {
    pub const fn new() -> Self {
        Self {
            root_path: "",
            listening_paths: &[""],
            excluded_paths: &["./target"],
            log: true,
            rebuild_with: None,
            phantom: std::marker::PhantomData,
        }
    }
}

impl<Ctx: HotReloadingContext> Config<Ctx> {
    /// Set the root path of the project (where the Cargo.toml file is). This is automatically set by the [`hot_reload_init`] macro.
    pub fn root(self, path: &'static str) -> Self {
        Self {
            root_path: path,
            ..self
        }
    }

    /// Set whether to enable logs
    pub fn with_logging(self, log: bool) -> Self {
        Self { log, ..self }
    }

    /// Set the command to run to rebuild the project
    ///
    /// For example to restart the application after a change is made, you could use `cargo run`
    pub fn with_rebuild_command(self, rebuild_command: &'static str) -> Self {
        self.with_rebuild_callback(move || {
            execute::shell(rebuild_command)
                .spawn()
                .expect("Failed to spawn the rebuild command");
            true
        })
    }

    /// Set a callback to run to when the project needs to be rebuilt and returns if the server should shut down
    ///
    /// For example a CLI application could rebuild the application when a change is made
    pub fn with_rebuild_callback(
        self,
        rebuild_callback: impl FnMut() -> bool + Send + 'static,
    ) -> Self {
        Self {
            rebuild_with: Some(Box::new(rebuild_callback)),
            ..self
        }
    }

    /// Set the paths to listen for changes in to trigger hot reloading. If this is a directory it will listen for changes in all files in that directory recursively.
    pub fn with_paths(self, paths: &'static [&'static str]) -> Self {
        Self {
            listening_paths: paths,
            ..self
        }
    }

    /// Sets paths to ignore changes on. This will override any paths set in the [`Config::with_paths`] method in the case of conflicts.
    pub fn excluded_paths(self, paths: &'static [&'static str]) -> Self {
        Self {
            excluded_paths: paths,
            ..self
        }
    }
}

/// Initialize the hot reloading listener
///
/// This is designed to be called by hot_reload_Init!() which will pass in information about the project
///
/// Notes:
/// - We don't wannt to watch the
pub fn init<Ctx: HotReloadingContext + Send + 'static>(cfg: Config<Ctx>) {
    let Config {
        mut rebuild_with,
        root_path,
        listening_paths,
        log,
        excluded_paths,
        ..
    } = cfg;

    let Ok(crate_dir) = PathBuf::from_str(root_path) else {
        return;
    };

    // try to find the gitignore file
    let gitignore_file_path = crate_dir.join(".gitignore");
    let (gitignore, _) = ignore::gitignore::Gitignore::new(gitignore_file_path);

    // convert the excluded paths to absolute paths
    let excluded_paths = excluded_paths
        .iter()
        .map(|path| crate_dir.join(PathBuf::from(path)))
        .collect::<Vec<_>>();

    let channels = Arc::new(Mutex::new(Vec::new()));
    let FileMapBuildResult {
        map: file_map,
        errors,
    } = FileMap::<Ctx>::create_with_filter(crate_dir.clone(), |path| {
        // skip excluded paths
        excluded_paths.iter().any(|p| path.starts_with(p)) ||
            // respect .gitignore
            gitignore
                .matched_path_or_any_parents(path, path.is_dir())
                .is_ignore()
    })
    .unwrap();

    for err in errors {
        if log {
            println!("hot reloading failed to initialize:\n{err:?}");
        }
    }

    let file_map = Arc::new(Mutex::new(file_map));

    let target_dir = crate_dir.join("target");
    let hot_reload_socket_path = target_dir.join("dioxusin");

    #[cfg(unix)]
    {
        // On unix, if you force quit the application, it can leave the file socket open
        // This will cause the local socket listener to fail to open
        // We check if the file socket is already open from an old session and then delete it
        if hot_reload_socket_path.exists() {
            let _ = std::fs::remove_file(hot_reload_socket_path.clone());
        }
    }

    let listener = if cfg!(windows) {
        LocalSocketListener::bind("@dioxusin")
    } else {
        LocalSocketListener::bind(hot_reload_socket_path)
    };

    let local_socket_stream = match listener {
        Ok(local_socket_stream) => local_socket_stream,
        Err(err) => {
            println!("failed to connect to hot reloading\n{err}");
            return;
        }
    };

    let aborted = Arc::new(Mutex::new(false));

    // listen for connections
    std::thread::spawn({
        let file_map = file_map.clone();
        let channels = channels.clone();
        let aborted = aborted.clone();
        let _ = local_socket_stream.set_nonblocking(true);
        move || {
            loop {
                if let Ok(mut connection) = local_socket_stream.accept() {
                    // send any templates than have changed before the socket connected
                    let templates: Vec<_> = {
                        file_map
                            .lock()
                            .unwrap()
                            .map
                            .values()
                            .flat_map(|v| v.templates.values().copied())
                            .collect()
                    };

                    for template in templates {
                        if !send_msg(HotReloadMsg::UpdateTemplate(template), &mut connection) {
                            continue;
                        }
                    }
                    channels.lock().unwrap().push(connection);
                    if log {
                        println!("Connected to hot reloading 🚀");
                    }
                }
                if *aborted.lock().unwrap() {
                    break;
                }
            }
        }
    });

    // watch for changes
    std::thread::spawn(move || {
        let mut last_update_time = chrono::Local::now().timestamp();

        let (tx, rx) = std::sync::mpsc::channel();

        let mut watcher = RecommendedWatcher::new(tx, notify::Config::default()).unwrap();

        let mut listening_pathbufs = vec![];

        // We're attempting to watch the root path... which contains a target directory...
        // And on some platforms the target directory is really really large and can cause the watcher to crash
        // since it runs out of file handles
        // So we're going to iterate through its children and watch them instead of the root path, skipping the target
        // directory.
        //
        // In reality, this whole approach of doing embedded file watching is kinda hairy since you want full knowledge
        // of where rust code is. We could just use the filemap we generated above as an indication of where the rust
        // code is in this project and deduce the subfolders under the root path from that.
        //
        // FIXME: use a more robust system here for embedded discovery
        //
        // https://github.com/DioxusLabs/dioxus/issues/1914
        if listening_paths == [""] {
            for entry in std::fs::read_dir(&crate_dir)
                .expect("failed to read rust crate directory. Are you running with cargo?")
            {
                let entry = entry.expect("failed to read directory entry");
                let path = entry.path();
                if path.is_dir() {
                    if path == target_dir {
                        continue;
                    }
                    listening_pathbufs.push(path);
                }
            }
        } else {
            for path in listening_paths {
                let full_path = crate_dir.join(path);
                listening_pathbufs.push(full_path);
            }
        }

        for full_path in listening_pathbufs {
            if let Err(err) = watcher.watch(&full_path, RecursiveMode::Recursive) {
                if log {
                    println!("hot reloading failed to start watching {full_path:?}:\n{err:?}",);
                }
            }
        }

        let mut rebuild = {
            let aborted = aborted.clone();
            let channels = channels.clone();
            move || {
                if let Some(rebuild_callback) = &mut rebuild_with {
                    if log {
                        println!("Rebuilding the application...");
                    }
                    let shutdown = rebuild_callback();

                    if shutdown {
                        *aborted.lock().unwrap() = true;
                    }

                    for channel in &mut *channels.lock().unwrap() {
                        send_msg(HotReloadMsg::Shutdown, channel);
                    }

                    return shutdown;
                } else if log {
                    println!("Rebuild needed... shutting down hot reloading.\nManually rebuild the application to view further changes.");
                }
                true
            }
        };

        for evt in rx {
            if chrono::Local::now().timestamp_millis() < last_update_time {
                continue;
            }

            let Ok(evt) = evt else {
                continue;
            };

            let real_paths = evt
                .paths
                .iter()
                .filter(|path| {
                    // skip non rust files
                    matches!(
                        path.extension().and_then(|p| p.to_str()),
                        Some("rs" | "toml" | "css" | "html" | "js")
                    ) &&
                    // skip excluded paths
                    !excluded_paths.iter().any(|p| path.starts_with(p)) &&
                    // respect .gitignore
                    !gitignore
                        .matched_path_or_any_parents(path, false)
                        .is_ignore()
                })
                .collect::<Vec<_>>();

            // Give time for the change to take effect before reading the file
            if !real_paths.is_empty() {
                std::thread::sleep(std::time::Duration::from_millis(10));
            }

            let mut channels = channels.lock().unwrap();
            for path in real_paths {
                // if this file type cannot be hot reloaded, rebuild the application
                if path.extension().and_then(|p| p.to_str()) != Some("rs") && rebuild() {
                    return;
                }
                // find changes to the rsx in the file
                let changes = file_map
                    .lock()
                    .unwrap()
                    .update_rsx(path, crate_dir.as_path());

                match changes {
                    Ok(UpdateResult::UpdatedRsx(msgs)) => {
                        for msg in msgs {
                            let mut i = 0;
                            while i < channels.len() {
                                let channel = &mut channels[i];
                                if send_msg(HotReloadMsg::UpdateTemplate(msg), channel) {
                                    i += 1;
                                } else {
                                    channels.remove(i);
                                }
                            }
                        }
                    }

                    Ok(UpdateResult::NeedsRebuild) => {
                        drop(channels);
                        if rebuild() {
                            return;
                        }
                        break;
                    }
                    Err(err) => {
                        if log {
                            println!("hot reloading failed to update rsx:\n{err:?}");
                        }
                    }
                }
            }

            last_update_time = chrono::Local::now().timestamp_millis();
        }
    });
}

fn send_msg(msg: HotReloadMsg, channel: &mut impl Write) -> bool {
    if let Ok(msg) = serde_json::to_string(&msg) {
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
