use crate::server::Diagnostic;
use colored::Colorize;
use dioxus_cli_config::crate_root;
use dioxus_cli_config::CrateConfig;
use std::path::PathBuf;
use std::process::Command;

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
        "Default"
    };
    let url_rewrite = if config.dioxus_config.web.watcher.index_on_404 {
        "True"
    } else {
        "False"
    };

    let proxies = &config.dioxus_config.web.proxy;

    if options.changed.is_empty() {
        println!(
            "{} @ v{} [{}] \n",
            "Dioxus".bold().green(),
            crate::DIOXUS_CLI_VERSION,
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
                "\t> Local : {}",
                format!("https://localhost:{}/", port).blue()
            );
            println!(
                "\t> Network : {}",
                format!("https://{}:{}/", ip, port).blue()
            );
            println!("\t> HTTPS : {}", "Enabled".to_string().green());
        } else {
            println!(
                "\t> Local : {}",
                format!("http://localhost:{}/", port).blue()
            );
            println!(
                "\t> Network : {}",
                format!("http://{}:{}/", ip, port).blue()
            );
            println!("\t> HTTPS : {}", "Disabled".to_string().red());
        }
    }
    println!();
    println!("\t> Profile : {}", profile.green());
    println!("\t> Hot Reload : {}", hot_reload.cyan());
    if !proxies.is_empty() {
        println!("\t> Proxies :");
        for proxy in proxies {
            println!("\t\t- {}", proxy.backend.blue());
        }
    }
    println!("\t> Index Template : {}", custom_html_file.green());
    println!("\t> URL Rewrite [index_on_404] : {}", url_rewrite.purple());
    println!();
    println!(
        "\t> Build Time Use : {} millis",
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
