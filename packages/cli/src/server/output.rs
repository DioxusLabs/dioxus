use crate::server::Diagnostic;
use colored::Colorize;
use dioxus_cli_config::{crate_root, CrateConfig};
use std::{path::PathBuf, process::Command};

#[derive(Debug, Default)]
pub struct PrettierOptions {
    pub changed: Vec<PathBuf>,
    pub warnings: Vec<Diagnostic>,
    pub elapsed_time: u128,
}

#[derive(Debug, Clone)]
pub struct WebServerInfo {
    pub ip: String,
    pub port: u16,
}

pub fn print_console_info(
    config: &CrateConfig,
    options: PrettierOptions,
    web_info: Option<WebServerInfo>,
) {
    // Don't clear the screen if the user has set the DIOXUS_LOG environment variable to "trace" so that we can see the logs
    if Some("trace") != std::env::var("DIOXUS_LOG").ok().as_deref() {
        if let Ok(native_clearseq) = Command::new(if cfg!(target_os = "windows") {
            "cls"
        } else {
            "clear"
        })
        .output()
        {
            print!("{}", String::from_utf8_lossy(&native_clearseq.stdout));
        } else {
            // Try ANSI-Escape characters
            print!("\x1b[2J\x1b[H");
        }
    }

    let mut profile = if config.release { "Release" } else { "Debug" }.to_string();
    if config.custom_profile.is_some() {
        profile = config.custom_profile.as_ref().unwrap().to_string();
    }
    let hot_reload = if config.hot_reload { "RSX" } else { "Normal" };
    let crate_root = crate_root().unwrap();
    let custom_html_file = if crate_root.join("index.html").is_file() {
        "Custom [index.html]"
    } else {
        "None"
    };
    let url_rewrite = if config.dioxus_config.web.watcher.index_on_404 {
        "True"
    } else {
        "False"
    };

    let proxies = &config.dioxus_config.web.proxy;

    if options.changed.is_empty() {
        println!(
            "{} @ v{} [{}]",
            "Dioxus".bold().green(),
            clap::crate_version!(),
            chrono::Local::now().format("%H:%M:%S").to_string().dimmed()
        );
    } else {
        println!(
            "Project Reloaded: {}\n",
            format!(
                "Changed {} files. [{}]",
                options.changed.len(),
                chrono::Local::now().format("%H:%M:%S").to_string().dimmed()
            )
            .purple()
            .bold()
        );
    }

    if let Some(WebServerInfo { ip, port }) = web_info {
        if config.dioxus_config.web.https.enabled == Some(true) {
            println!(
                "    > Local address: {}",
                format!("https://localhost:{}/", port).blue()
            );
            println!(
                "    > Network address: {}",
                format!("https://{}:{}/", ip, port).blue()
            );
            println!("    > HTTPS: {}", "Enabled".to_string().green());
        } else {
            println!(
                "    > Local address: {}",
                format!("http://localhost:{}/", port).blue()
            );
            println!(
                "    > Network address: {}",
                format!("http://{}:{}/", ip, port).blue()
            );
            println!("    > HTTPS status: {}", "Disabled".to_string().red());
        }
    }
    println!();

    println!("    > Hot Reload Mode: {}", hot_reload.cyan());

    println!(
        "    > Watching: [ {} ]",
        config
            .dioxus_config
            .web
            .watcher
            .watch_path
            .iter()
            .cloned()
            .chain(["Cargo.toml", "Dioxus.toml"].iter().map(PathBuf::from))
            .map(|f| f.display().to_string())
            .collect::<Vec<String>>()
            .join(", ")
            .cyan()
    );

    if !proxies.is_empty() {
        println!("    > Proxies :");
        for proxy in proxies {
            println!("    - {}", proxy.backend.blue());
        }
    }
    println!("    > Custom index.html: {}", custom_html_file.green());
    println!("    > Serve index.html on 404: {}", url_rewrite.purple());
    println!();
    println!(
        "    > Build Features: [ {} ]",
        config
            .features
            .clone()
            .unwrap_or_default()
            .join(", ")
            .green()
    );
    println!("    > Build Profile: {}", profile.green());
    println!(
        "    > Build took: {} millis",
        options.elapsed_time.to_string().green().bold()
    );
    println!();

    if options.warnings.is_empty() {
        log::info!("{}\n", "A perfect compilation!".green().bold());
    } else {
        log::warn!(
            "{}",
            format!(
                "There were {} warning messages during the build. Run `cargo check` to see them.",
                options.warnings.len() - 1
            )
            .yellow()
            .bold()
        );
    }
}
