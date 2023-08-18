use crate::{
    cfg::ConfigOptsServe,
    server::{
        output::{print_console_info, PrettierOptions},
        setup_file_watcher, setup_file_watcher_hot_reload,
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

use super::Platform;

pub async fn startup(config: CrateConfig, serve: &ConfigOptsServe) -> Result<()> {
    startup_with_platform::<DesktopPlatform>(config, serve).await
}

pub(crate) async fn startup_with_platform<P: Platform + Send + 'static>(
    config: CrateConfig,
    serve: &ConfigOptsServe,
) -> Result<()> {
    // ctrl-c shutdown checker
    let _crate_config = config.clone();
    let _ = ctrlc::set_handler(move || {
        #[cfg(feature = "plugin")]
        let _ = PluginManager::on_serve_shutdown(&_crate_config);
        std::process::exit(0);
    });

    match config.hot_reload {
        true => serve_hot_reload::<P>(config, serve).await?,
        false => serve_default::<P>(config, serve).await?,
    }

    Ok(())
}

/// Start the server without hot reload
async fn serve_default<P: Platform + Send + 'static>(
    config: CrateConfig,
    serve: &ConfigOptsServe,
) -> Result<()> {
    let platform = RwLock::new(P::start(&config, serve)?);

    log::info!("🚀 Starting development server...");

    // We got to own watcher so that it exists for the duration of serve
    // Otherwise full reload won't work.
    let _watcher = setup_file_watcher(
        {
            let config = config.clone();

            move || platform.write().unwrap().rebuild(&config)
        },
        &config,
        None,
    )
    .await?;

    std::future::pending::<()>().await;

    Ok(())
}

/// Start dx serve with hot reload
async fn serve_hot_reload<P: Platform + Send + 'static>(
    config: CrateConfig,
    serve: &ConfigOptsServe,
) -> Result<()> {
    let platform = RwLock::new(P::start(&config, serve)?);

    // Setup hot reload
    let FileMapBuildResult { map, errors } =
        FileMap::<HtmlCtx>::create(config.crate_dir.clone()).unwrap();

    for err in errors {
        log::error!("{}", err);
    }

    let file_map = Arc::new(Mutex::new(map));

    let (hot_reload_tx, mut hot_reload_rx) = broadcast::channel(100);

    // States
    // The open interprocess sockets
    let channels = Arc::new(Mutex::new(Vec::new()));

    // Setup file watcher
    // We got to own watcher so that it exists for the duration of serve
    // Otherwise hot reload won't work.
    let _watcher = setup_file_watcher_hot_reload(
        &config,
        hot_reload_tx,
        file_map.clone(),
        {
            let config = config.clone();

            let channels = channels.clone();
            move || {
                for channel in &mut *channels.lock().unwrap() {
                    send_msg(HotReloadMsg::Shutdown, channel);
                }
                Ok(platform.write().unwrap().rebuild(&config)?)
            }
        },
        None,
    )
    .await?;

    clear_paths();

    match LocalSocketListener::bind("@dioxusin") {
        Ok(local_socket_stream) => {
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

fn start_desktop(config: &CrateConfig) -> Result<(Child, BuildResult)> {
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

pub(crate) struct DesktopPlatform {
    currently_running_child: Child,
}

impl Platform for DesktopPlatform {
    fn start(config: &CrateConfig, _serve: &ConfigOptsServe) -> Result<Self> {
        let (child, first_build_result) = start_desktop(&config)?;

        log::info!("🚀 Starting development server...");

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

        Ok(Self {
            currently_running_child: child,
        })
    }

    fn rebuild(&mut self, config: &CrateConfig) -> Result<BuildResult> {
        self.currently_running_child.kill()?;
        let (child, result) = start_desktop(&config)?;
        self.currently_running_child = child;
        Ok(result)
    }
}
