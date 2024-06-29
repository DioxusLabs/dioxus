use crate::{
    builder,
    cfg::ConfigOptsServe,
    serve::Serve,
    server::{
        output::{print_console_info, PrettierOptions, WebServerInfo},
        setup_file_watcher,
    },
    BuildResult, Result,
};
use dioxus_cli_config::CrateConfig;
use dioxus_rsx::hot_reload::*;
use std::{
    net::{IpAddr, SocketAddr, UdpSocket},
    sync::Arc,
};

mod proxy;
mod server;

use server::*;

use super::HotReloadState;

pub fn startup(config: CrateConfig, serve_cfg: &ConfigOptsServe) -> Result<()> {
    set_ctrlc_handler(&config);

    let ip = serve_cfg
        .server_arguments
        .addr
        .or_else(get_ip)
        .unwrap_or(IpAddr::V4(std::net::Ipv4Addr::new(0, 0, 0, 0)));

    let hot_reload_state = build_hotreload_filemap(&config);

    serve(ip, config, hot_reload_state, serve_cfg)
}

/// Start the server without hot reload
pub fn serve(
    ip: IpAddr,
    config: CrateConfig,
    hot_reload_state: HotReloadState,
    opts: &ConfigOptsServe,
) -> Result<()> {
    let skip_assets = opts.skip_assets;
    let port = opts.server_arguments.port;

    // Since web platform doesn't use `rust_flags`, this argument is explicitly
    // set to `None`.
    let first_build_result = crate::builder::build_web(&config, skip_assets, None)?;

    // generate dev-index page
    Serve::regen_dev_page(&config, first_build_result.assets.as_ref())?;

    tracing::info!("🚀 Starting development server...");

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async move {
        // We got to own watcher so that it exists for the duration of serve
        // Otherwise full reload won't work.
        let _watcher = setup_file_watcher(
            {
                let config = config.clone();
                let hot_reload_state = hot_reload_state.clone();
                move || build(&config, &hot_reload_state, skip_assets)
            },
            &config,
            Some(WebServerInfo { ip, port }),
            hot_reload_state.clone(),
        )
        .await?;

        // HTTPS
        // Before console info so it can stop if mkcert isn't installed or fails
        let rustls_config = get_rustls(&config).await?;

        // Print serve info
        print_console_info(
            &config,
            PrettierOptions {
                changed: vec![],
                warnings: first_build_result.warnings,
                elapsed_time: first_build_result.elapsed_time,
            },
            Some(WebServerInfo { ip, port }),
        );

        // Router
        let router = setup_router(config.clone(), hot_reload_state).await?;

        // Start server
        start_server(ip, port, router, opts.open, rustls_config, &config).await?;

        Ok(())
    })
}

/// Starts dx serve with no hot reload
async fn start_server(
    ip: IpAddr,
    port: u16,
    router: axum::Router,
    start_browser: bool,
    rustls: Option<axum_server::tls_rustls::RustlsConfig>,
    config: &CrateConfig,
) -> Result<()> {
    // If plugins, call on_serve_start event
    #[cfg(feature = "plugin")]
    crate::plugin::PluginManager::on_serve_start(config)?;

    let addr: SocketAddr = SocketAddr::from((ip, port));

    // Open the browser
    if start_browser {
        open_browser(config, ip, port, rustls.is_some());
    }

    let svc = router.into_make_service();

    // Start the server with or without rustls
    match rustls {
        Some(rustls) => axum_server::bind_rustls(addr, rustls).serve(svc).await?,
        None => {
            // Create a TCP listener bound to the address
            let listener = tokio::net::TcpListener::bind(&addr).await?;
            axum::serve(listener, svc).await?
        }
    }

    Ok(())
}

/// Open the browser to the address
pub(crate) fn open_browser(config: &CrateConfig, ip: IpAddr, port: u16, https: bool) {
    let protocol = if https { "https" } else { "http" };
    let base_path = match config.dioxus_config.web.app.base_path.as_deref() {
        Some(base_path) => format!("/{}", base_path.trim_matches('/')),
        None => "".to_owned(),
    };
    _ = open::that(format!("{protocol}://{ip}:{port}{base_path}"));
}

/// Get the network ip
fn get_ip() -> Option<IpAddr> {
    let socket = match UdpSocket::bind("0.0.0.0:0") {
        Ok(s) => s,
        Err(_) => return None,
    };

    match socket.connect("8.8.8.8:80") {
        Ok(()) => (),
        Err(_) => return None,
    };

    match socket.local_addr() {
        Ok(addr) => Some(addr.ip()),
        Err(_) => None,
    }
}

fn build(
    config: &CrateConfig,
    hot_reload_state: &HotReloadState,
    skip_assets: bool,
) -> Result<BuildResult> {
    // Since web platform doesn't use `rust_flags`, this argument is explicitly
    // set to `None`.
    let result = std::panic::catch_unwind(|| builder::build_web(config, skip_assets, None))
        .map_err(|e| anyhow::anyhow!("Build failed: {e:?}"))?;

    // change the websocket reload state to true;
    // the page will auto-reload.
    if config.dioxus_config.web.watcher.reload_html {
        if let Ok(assets) = result.as_ref().map(|x| x.assets.as_ref()) {
            let _ = Serve::regen_dev_page(config, assets);
        }
    }

    hot_reload_state.receiver.reload();

    result
}

fn set_ctrlc_handler(config: &CrateConfig) {
    // ctrl-c shutdown checker
    let _crate_config = config.clone();

    let _ = ctrlc::set_handler(move || {
        #[cfg(feature = "plugin")]
        let _ = crate::plugin::PluginManager::on_serve_shutdown(&_crate_config);

        std::process::exit(0);
    });
}

fn build_hotreload_filemap(config: &CrateConfig) -> HotReloadState {
    HotReloadState {
        file_map: config.hot_reload.then(|| {
            let FileMapBuildResult { map, errors } =
                FileMap::create(config.crate_dir.clone()).unwrap();

            for err in errors {
                tracing::error!("{}", err);
            }

            Arc::new(Mutex::new(map))
        }),
        receiver: Default::default(),
    }
}
