use crate::{builder::BuildConfig, cli::DevelopOptions, config::CrateConfig, error::Result};
use async_std::prelude::FutureExt;

use async_std::future;
use async_std::prelude::*;

use log::info;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;

pub struct DevelopConfig {}
impl Into<DevelopConfig> for DevelopOptions {
    fn into(self) -> DevelopConfig {
        DevelopConfig {}
    }
}

type ErrStatus = Arc<AtomicBool>;

pub async fn start(config: &CrateConfig, _options: &DevelopConfig) -> Result<()> {
    log::info!("Starting development server 🚀");

    let CrateConfig { out_dir, .. } = config;

    let is_err = Arc::new(AtomicBool::new(false));

    // Spawn the server onto a seperate task
    // This lets the task progress while we handle file updates
    let server = async_std::task::spawn(launch_server(out_dir.clone(), is_err.clone()));

    let watcher = async_std::task::spawn(watch_directory(config.clone(), is_err.clone()));

    match server.race(watcher).await {
        Err(e) => log::warn!("Error running development server, {:?}", e),
        _ => {}
    }

    Ok(())
}

async fn watch_directory(config: CrateConfig, is_err: ErrStatus) -> Result<()> {
    // Create a channel to receive the events.
    let (watcher_tx, watcher_rx) = async_std::channel::bounded(100);

    // Automatically select the best implementation for your platform.
    // You can also access each implementation directly e.g. INotifyWatcher.
    let mut watcher: RecommendedWatcher = Watcher::new(move |res| {
<<<<<<< HEAD
        async_std::task::block_on(watcher_tx.send(res));
=======
        // send an event
        let _ = async_std::task::block_on(watcher_tx.send(res));
>>>>>>> 9451713 (wip: studio upgrades)
    })
    .expect("failed to make watcher");

    let src_dir = crate::cargo::crate_root()?;

    // Add a path to be watched. All files and directories at that path and
    // below will be monitored for changes.
    watcher
        .watch(&src_dir.join("src"), RecursiveMode::Recursive)
        .expect("Failed to watch dir");

    match watcher.watch(&src_dir.join("examples"), RecursiveMode::Recursive) {
        Ok(_) => {}
        Err(e) => log::warn!("Failed to watch examples dir, {:?}", e),
    }

    let build_config = BuildConfig::default();

    'run: loop {
        match crate::builder::build(&config, &build_config) {
            Ok(_) => {
                is_err.store(false, std::sync::atomic::Ordering::Relaxed);
                async_std::task::sleep(std::time::Duration::from_millis(500)).await;
            }
            Err(err) => is_err.store(true, std::sync::atomic::Ordering::Relaxed),
        };

        let mut msg = None;
        loop {
            let new_msg = watcher_rx.recv().await.unwrap().unwrap();
            if !watcher_rx.is_empty() {
                msg = Some(new_msg);
                break;
            }
        }

        info!("File updated, rebuilding app");
    }
    Ok(())
}

async fn launch_server(outdir: PathBuf, is_err: ErrStatus) -> Result<()> {
    let _crate_dir = crate::cargo::crate_root()?;
    let _workspace_dir = crate::cargo::workspace_root()?;

    let mut app = tide::with_state(ServerState::new(outdir.to_owned(), is_err));
    let p = outdir.display().to_string();

    app.at("/")
        .get(|req: tide::Request<ServerState>| async move {
            log::info!("Connected to development server");
            let state = req.state();

            match state.is_err.load(std::sync::atomic::Ordering::Relaxed) {
                true => Ok(tide::Body::from_string(format!(
                    include_str!("./err.html"),
                    err = "_"
                ))),
                false => {
                    Ok(tide::Body::from_file(state.serv_path.clone().join("index.html")).await?)
                }
            }
        })
        .serve_dir(p)?;

    let port = "8080";
    // let serve_addr = format!("0.0.0.0:{}", port);
    let serve_addr = format!("127.0.0.1:{}", port);

    info!("App available at http://{}", serve_addr);
    app.listen(serve_addr).await?;
    Ok(())
}

/// https://github.com/http-rs/tide/blob/main/examples/state.rs
/// Tide seems to prefer using state instead of injecting into the app closure
/// The app closure needs to be static and
#[derive(Clone)]
struct ServerState {
    serv_path: PathBuf,
    is_err: ErrStatus,
}
impl ServerState {
    fn new(serv_path: PathBuf, is_err: ErrStatus) -> Self {
        Self { serv_path, is_err }
    }
}
