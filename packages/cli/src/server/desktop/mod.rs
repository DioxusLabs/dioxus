use crate::{
    server::{
        output::{print_console_info, PrettierOptions},
        setup_file_watcher,
    },
    BuildResult, CrateConfig, Result,
};

use dioxus_hot_reload::HotReloadMsg;
use dioxus_html::HtmlCtx;
use dioxus_rsx::hot_reload::*;
use interprocess_docfix::local_socket::LocalSocketListener;
use std::{
    process::{Child, Command},
    sync::{Arc, Mutex, RwLock},
};
use tokio::sync::broadcast::{self};

#[cfg(feature = "plugin")]
use plugin::PluginManager;

use super::HotReloadState;

pub async fn startup(config: CrateConfig) -> Result<()> {
    // ctrl-c shutdown checker
    let _crate_config = config.clone();
    let _ = ctrlc::set_handler(move || {
        #[cfg(feature = "plugin")]
        let _ = PluginManager::on_serve_shutdown(&_crate_config);
        std::process::exit(0);
    });

    let hot_reload_state = match config.hot_reload {
        true => {
            let FileMapBuildResult { map, errors } =
                FileMap::<HtmlCtx>::create(config.crate_dir.clone()).unwrap();

            for err in errors {
                log::error!("{}", err);
            }

            let file_map = Arc::new(Mutex::new(map));

            let hot_reload_tx = broadcast::channel(100).0;

            clear_paths();

            Some(HotReloadState {
                messages: hot_reload_tx.clone(),
                file_map: file_map.clone(),
            })
        }
        false => None,
    };

    serve(config, hot_reload_state).await?;

    Ok(())
}

/// Start the server without hot reload
pub async fn serve(config: CrateConfig, hot_reload_state: Option<HotReloadState>) -> Result<()> {
    let (child, first_build_result) = start_desktop(&config)?;
    let currently_running_child: RwLock<Child> = RwLock::new(child);

    log::info!("🚀 Starting development server...");

    // We got to own watcher so that it exists for the duration of serve
    // Otherwise full reload won't work.
    let _watcher = setup_file_watcher(
        {
            let config = config.clone();

            move || {
                let mut current_child = currently_running_child.write().unwrap();
                current_child.kill()?;
                let (child, result) = start_desktop(&config)?;
                *current_child = child;
                Ok(result)
            }
        },
        &config,
        None,
        hot_reload_state.clone(),
    )
    .await?;

    // Print serve info
    print_console_info(
        &config,
        PrettierOptions {
            changed: vec![],
            warnings: first_build_result.warnings,
            elapsed_time: first_build_result.elapsed_time,
        },
        None,
    );

    match hot_reload_state {
        Some(hot_reload_state) => {
            start_desktop_hot_reload(hot_reload_state).await?;
        }
        None => {
            std::future::pending::<()>().await;
        }
    }

    Ok(())
}

async fn start_desktop_hot_reload(hot_reload_state: HotReloadState) -> Result<()> {
    match LocalSocketListener::bind("@dioxusin") {
        Ok(local_socket_stream) => {
            let aborted = Arc::new(Mutex::new(false));
            // States
            // The open interprocess sockets
            let channels = Arc::new(Mutex::new(Vec::new()));

            // listen for connections
            std::thread::spawn({
                let file_map = hot_reload_state.file_map.clone();
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
                                    .filter_map(|(_, template_slot)| *template_slot)
                                    .collect()
                            };
                            for template in templates {
                                if !send_msg(
                                    HotReloadMsg::UpdateTemplate(template),
                                    &mut connection,
                                ) {
                                    continue;
                                }
                            }
                            channels.lock().unwrap().push(connection);
                            println!("Connected to hot reloading 🚀");
                        }
                        if *aborted.lock().unwrap() {
                            break;
                        }
                    }
                }
            });

            let mut hot_reload_rx = hot_reload_state.messages.subscribe();

            while let Ok(template) = hot_reload_rx.recv().await {
                let channels = &mut *channels.lock().unwrap();
                let mut i = 0;
                while i < channels.len() {
                    let channel = &mut channels[i];
                    if send_msg(HotReloadMsg::UpdateTemplate(template), channel) {
                        i += 1;
                    } else {
                        channels.remove(i);
                    }
                }
            }
        }
        Err(error) => println!("failed to connect to hot reloading\n{error}"),
    }

    Ok(())
}

fn clear_paths() {
    if cfg!(target_os = "macos") {
        // On unix, if you force quit the application, it can leave the file socket open
        // This will cause the local socket listener to fail to open
        // We check if the file socket is already open from an old session and then delete it
        let paths = ["./dioxusin", "./@dioxusin"];
        for path in paths {
            let path = std::path::PathBuf::from(path);
            if path.exists() {
                let _ = std::fs::remove_file(path);
            }
        }
    }
}

fn send_msg(msg: HotReloadMsg, channel: &mut impl std::io::Write) -> bool {
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

pub fn start_desktop(config: &CrateConfig) -> Result<(Child, BuildResult)> {
    // Run the desktop application
    let result = crate::builder::build_desktop(config, true)?;

    match &config.executable {
        crate::ExecutableType::Binary(name)
        | crate::ExecutableType::Lib(name)
        | crate::ExecutableType::Example(name) => {
            let mut file = config.out_dir.join(name);
            if cfg!(windows) {
                file.set_extension("exe");
            }
            let child = Command::new(file.to_str().unwrap()).spawn()?;

            Ok((child, result))
        }
    }
}
