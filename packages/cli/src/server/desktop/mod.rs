use crate::server::Platform;
use crate::{
    cfg::ConfigOptsServe,
    server::{
        output::{print_console_info, PrettierOptions},
        setup_file_watcher,
    },
    BuildResult, Result,
};
use dioxus_cli_config::CrateConfig;
use dioxus_cli_config::ExecutableType;

use dioxus_hot_reload::HotReloadMsg;
use dioxus_html::HtmlCtx;
use dioxus_rsx::hot_reload::*;
use interprocess_docfix::local_socket::LocalSocketListener;
use std::fs::create_dir_all;
use std::{
    process::{Child, Command},
    sync::{Arc, Mutex, RwLock},
};
use tokio::sync::broadcast::{self};

#[cfg(feature = "plugin")]
use crate::plugin::PluginManager;

use super::HotReloadState;

pub async fn startup(config: CrateConfig, serve: &ConfigOptsServe) -> Result<()> {
    startup_with_platform::<DesktopPlatform>(config, serve).await
}

pub(crate) async fn startup_with_platform<P: Platform + Send + 'static>(
    config: CrateConfig,
    serve_cfg: &ConfigOptsServe,
) -> Result<()> {
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

            Some(HotReloadState {
                messages: hot_reload_tx.clone(),
                file_map: file_map.clone(),
            })
        }
        false => None,
    };

    serve::<P>(config, serve_cfg, hot_reload_state).await?;

    Ok(())
}

/// Start the server without hot reload
async fn serve<P: Platform + Send + 'static>(
    config: CrateConfig,
    serve: &ConfigOptsServe,
    hot_reload_state: Option<HotReloadState>,
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
        hot_reload_state.clone(),
    )
    .await?;

    match hot_reload_state {
        Some(hot_reload_state) => {
            // The open interprocess sockets
            start_desktop_hot_reload(hot_reload_state).await?;
        }
        None => {
            std::future::pending::<()>().await;
        }
    }

    Ok(())
}

async fn start_desktop_hot_reload(hot_reload_state: HotReloadState) -> Result<()> {
    let metadata = cargo_metadata::MetadataCommand::new()
        .no_deps()
        .exec()
        .unwrap();
    let target_dir = metadata.target_directory.as_std_path();
    let _ = create_dir_all(target_dir); // `_all` is for good measure and future-proofness.
    let path = target_dir.join("dioxusin");
    clear_paths(&path);
    match LocalSocketListener::bind(path) {
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
                move || {
                    loop {
                        //accept() will block the thread when local_socket_stream is in blocking mode (default)
                        match local_socket_stream.accept() {
                            Ok(mut connection) => {
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
                            Err(err) => {
                                let error_string = err.to_string();
                                // Filter out any error messages about a operation that may block and an error message that triggers on some operating systems that says "Waiting for a process to open the other end of the pipe" without WouldBlock being set
                                let display_error = err.kind() != std::io::ErrorKind::WouldBlock
                                    && !error_string.contains("Waiting for a process");
                                if display_error {
                                    println!("Error connecting to hot reloading: {} (Hot reloading is a feature of the dioxus-cli. If you are not using the CLI, this error can be ignored)", err);
                                }
                            }
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

fn clear_paths(file_socket_path: &std::path::Path) {
    if cfg!(unix) {
        // On unix, if you force quit the application, it can leave the file socket open
        // This will cause the local socket listener to fail to open
        // We check if the file socket is already open from an old session and then delete it

        if file_socket_path.exists() {
            let _ = std::fs::remove_file(file_socket_path);
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

fn start_desktop(
    config: &CrateConfig,
    skip_assets: bool,
    rust_flags: Option<String>,
) -> Result<(RAIIChild, BuildResult)> {
    // Run the desktop application
    // Only used for the fullstack platform,
    let result = crate::builder::build_desktop(config, true, skip_assets, rust_flags)?;

    match &config.executable {
        ExecutableType::Binary(name)
        | ExecutableType::Lib(name)
        | ExecutableType::Example(name) => {
            let mut file = config.out_dir().join(name);
            if cfg!(windows) {
                file.set_extension("exe");
            }
            let active = "DIOXUS_ACTIVE";
            let child = RAIIChild(
                Command::new(file.to_str().unwrap())
                    .env(active, "true")
                    .spawn()?,
            );

            Ok((child, result))
        }
    }
}

pub(crate) struct DesktopPlatform {
    currently_running_child: RAIIChild,
    skip_assets: bool,
}

impl DesktopPlatform {
    /// `rust_flags` argument is added because it is used by the
    /// `DesktopPlatform`'s implementation of the `Platform::start()`.
    pub fn start_with_options(
        config: &CrateConfig,
        serve: &ConfigOptsServe,
        rust_flags: Option<String>,
    ) -> Result<Self> {
        let (child, first_build_result) = start_desktop(config, serve.skip_assets, rust_flags)?;

        log::info!("🚀 Starting development server...");

        // Print serve info
        print_console_info(
            config,
            PrettierOptions {
                changed: vec![],
                warnings: first_build_result.warnings,
                elapsed_time: first_build_result.elapsed_time,
            },
            None,
        );

        Ok(Self {
            currently_running_child: child,
            skip_assets: serve.skip_assets,
        })
    }

    /// `rust_flags` argument is added because it is used by the
    /// `DesktopPlatform`'s implementation of the `Platform::rebuild()`.
    pub fn rebuild_with_options(
        &mut self,
        config: &CrateConfig,
        rust_flags: Option<String>,
    ) -> Result<BuildResult> {
        self.currently_running_child.0.kill()?;
        let (child, result) = start_desktop(config, self.skip_assets, rust_flags)?;
        self.currently_running_child = child;
        Ok(result)
    }
}

impl Platform for DesktopPlatform {
    fn start(config: &CrateConfig, serve: &ConfigOptsServe) -> Result<Self> {
        // See `start_with_options()`'s docs for the explanation why the code
        // was moved there.
        // Since desktop platform doesn't use `rust_flags`, this argument is
        // explicitly set to `None`.
        DesktopPlatform::start_with_options(config, serve, None)
    }

    fn rebuild(&mut self, config: &CrateConfig) -> Result<BuildResult> {
        // See `rebuild_with_options()`'s docs for the explanation why the code
        // was moved there.
        // Since desktop platform doesn't use `rust_flags`, this argument is
        // explicitly set to `None`.
        DesktopPlatform::rebuild_with_options(self, config, None)
    }
}

struct RAIIChild(Child);

impl Drop for RAIIChild {
    fn drop(&mut self) {
        let _ = self.0.kill();
    }
}
