use crate::dioxus_crate::DioxusCrate;
use crate::serve::Serve;
use futures_util::StreamExt;

use super::{Builder, Server, Watcher};

pub struct Output {
    rx: futures_channel::mpsc::UnboundedReceiver<TuiInput>,
}

pub enum TuiInput {
    Shutdown,
    Keydown,
}

impl Output {
    pub fn start(cfg: &Serve, crate_config: &DioxusCrate) -> Self {
        // Wire the handler to ping the handle_input
        // This will give us some time to handle the input
        let (tx, rx) = futures_channel::mpsc::unbounded();
        ctrlc::set_handler(move || {
            tx.unbounded_send(TuiInput::Shutdown).unwrap();
        })
        .unwrap();

        Self { rx }
    }

    pub async fn wait(&mut self) -> TuiInput {
        // pending
        self.rx.next().await.unwrap()
    }

    pub fn handle_input(&mut self, input: TuiInput) {}

    pub fn draw(
        &self,
        opts: &Serve,
        config: &DioxusCrate,
        build_engine: &Builder,
        server: &Server,
        watcher: &Watcher,
    ) {
        // // Don't clear the screen if the user has set the DIOXUS_LOG environment variable to "trace" so that we can see the logs
        // if Some("trace") != std::env::var("DIOXUS_LOG").ok().as_deref() {
        //     if let Ok(native_clearseq) = Command::new(if cfg!(target_os = "windows") {
        //         "cls"
        //     } else {
        //         "clear"
        //     })
        //     .output()
        //     {
        //         print!("{}", String::from_utf8_lossy(&native_clearseq.stdout));
        //     } else {
        //         // Try ANSI-Escape characters
        //         print!("\x1b[2J\x1b[H");
        //     }
        // }

        // let mut profile = if config.release { "Release" } else { "Debug" }.to_string();
        // if config.custom_profile.is_some() {
        //     profile = config.custom_profile.as_ref().unwrap().to_string();
        // }
        // let hot_reload = if config.hot_reload { "RSX" } else { "Normal" };
        // let crate_root = crate_root().unwrap();
        // let custom_html_file = if crate_root.join("index.html").is_file() {
        //     "Custom [index.html]"
        // } else {
        //     "None"
        // };
        // let url_rewrite = if config.dioxus_config.web.watcher.index_on_404 {
        //     "True"
        // } else {
        //     "False"
        // };

        // let proxies = &config.dioxus_config.web.proxy;

        // if options.changed.is_empty() {
        //     println!(
        //         "{} @ v{} [{}]",
        //         "Dioxus".bold().green(),
        //         clap::crate_version!(),
        //         chrono::Local::now().format("%H:%M:%S").to_string().dimmed()
        //     );
        // } else {
        //     println!(
        //         "Project Reloaded: {}\n",
        //         format!(
        //             "Changed {} files. [{}]",
        //             options.changed.len(),
        //             chrono::Local::now().format("%H:%M:%S").to_string().dimmed()
        //         )
        //         .purple()
        //         .bold()
        //     );
        // }

        // if let Some(WebServerInfo { ip, port }) = web_info {
        //     let https = config.dioxus_config.web.https.enabled == Some(true);
        //     let prefix = if https { "https://" } else { "http://" };
        //     println!(
        //         "    > Local address: {}",
        //         format!("{prefix}localhost:{}/", port).blue()
        //     );
        //     println!(
        //         "    > Network address: {}",
        //         format!("{prefix}{}:{}/", ip, port).blue()
        //     );
        //     println!(
        //         "    > HTTPS: {}",
        //         if https {
        //             "Enabled".to_string().green()
        //         } else {
        //             "Disabled".to_string().red()
        //         }
        //     );
        // }
        // println!();

        // println!("    > Hot Reload Mode: {}", hot_reload.cyan());

        // println!(
        //     "    > Watching: [ {} ]",
        //     config
        //         .dioxus_config
        //         .web
        //         .watcher
        //         .watch_path
        //         .iter()
        //         .cloned()
        //         .chain(["Cargo.toml", "Dioxus.toml"].iter().map(PathBuf::from))
        //         .map(|f| f.display().to_string())
        //         .collect::<Vec<String>>()
        //         .join(", ")
        //         .cyan()
        // );

        // if !proxies.is_empty() {
        //     println!("    > Proxies :");
        //     for proxy in proxies {
        //         println!("    - {}", proxy.backend.blue());
        //     }
        // }
        // println!("    > Custom index.html: {}", custom_html_file.green());
        // println!("    > Serve index.html on 404: {}", url_rewrite.purple());
        // println!();
        // println!(
        //     "    > Build Features: [ {} ]",
        //     config
        //         .features
        //         .clone()
        //         .unwrap_or_default()
        //         .join(", ")
        //         .green()
        // );
        // println!("    > Build Profile: {}", profile.green());
        // println!(
        //     "    > Build took: {} millis",
        //     options.elapsed_time.to_string().green().bold()
        // );
        // println!();

        // if options.warnings.is_empty() {
        //     tracing::info!("{}\n", "A perfect compilation!".green().bold());
        // } else {
        //     tracing::warn!(
        //         "{}",
        //         format!(
        //         "There were {} warning messages during the build. Run `cargo check` to see them.",
        //         options.warnings.len() - 1
        //     )
        //         .yellow()
        //         .bold()
        //     );
        // }
    }
}

// use crate::server::Diagnostic;

// #[derive(Debug, Default)]
// pub struct PrettierOptions {
//     pub changed: Vec<PathBuf>,
//     pub warnings: Vec<Diagnostic>,
//     pub elapsed_time: u128,
// }

// #[derive(Debug, Clone)]
// pub struct WebServerInfo {
//     pub ip: IpAddr,
//     pub port: u16,
// }

// pub fn print_console_info(
//     config: &CrateConfig,
//     options: PrettierOptions,
//     web_info: Option<WebServerInfo>,
// ) {
// }
