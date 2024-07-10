use crate::{builder::UpdateStage, serve::Serve};
use crate::{
    builder::{BuildMessage, Stage, UpdateBuildProgress},
    dioxus_crate::DioxusCrate,
};
use crossterm::{
    event::{self, Event, EventStream, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    tty::IsTty,
    ExecutableCommand,
};
use dioxus_cli_config::Platform;
use futures_channel::mpsc::UnboundedReceiver;
use futures_util::{future::FutureExt, select, StreamExt};
use ratatui::{prelude::*, widgets::*, TerminalOptions, Viewport};
use std::{
    collections::{HashMap, VecDeque},
    io::{self, stdout},
};

use super::{Builder, Server, Watcher};

pub struct Output {
    term: TerminalBackend,
    events: EventStream,
    command_list: ListState,
    rustc_version: String,
    rustc_nightly: bool,
    dx_version: String,
    is_tty: bool,
    build_logs: HashMap<Platform, ActiveBuild>,
}

type TerminalBackend = Terminal<CrosstermBackend<io::Stdout>>;

impl Output {
    pub async fn start(cfg: &Serve, crate_config: &DioxusCrate) -> io::Result<Self> {
        let is_tty = std::io::stdout().is_tty();

        if is_tty {
            enable_raw_mode()?;
            stdout().execute(EnterAlternateScreen)?;
        }

        // set the panic hook to fix the terminal
        set_fix_term_hook();

        let events = EventStream::new();
        let term: TerminalBackend = Terminal::with_options(
            CrosstermBackend::new(stdout()),
            TerminalOptions {
                viewport: Viewport::Fullscreen,
            },
        )?;

        let command_list = ListState::default();

        let rustc_version = rustc_version().await;
        let rustc_nightly = rustc_version.contains("nightly") || cfg.target_args.nightly;

        let mut dx_version = String::new();

        if crate::dx_build_info::PROFILE != "release" {
            if let Some(hash) = crate::dx_build_info::GIT_COMMIT_HASH_SHORT {
                let hash = &hash.trim_start_matches("g")[..4];
                dx_version.push_str(hash);
            } else {
                dx_version.push_str(env!("CARGO_PKG_VERSION"));
            }
        }

        Ok(Self {
            term,
            events,
            command_list,
            rustc_version,
            rustc_nightly,
            dx_version,
            is_tty,
            build_logs: HashMap::new(),
        })
    }

    /// Wait for either the ctrl_c handler or the next event
    ///
    /// Why is the ctrl_c handler here?
    pub async fn wait(&mut self) -> Event {
        let event = self.events.next().await;
        event.unwrap().unwrap()
    }

    pub fn shutdown(&mut self) -> io::Result<()> {
        // if we're a tty then we need to disable the raw mode
        if self.is_tty {
            disable_raw_mode()?;
            stdout().execute(LeaveAlternateScreen)?;

            // todo: print the build info here for the most recent build, and then the logs of the most recent build
            println!("build output goes here");
            println!("build logs go here");
        }

        Ok(())
    }

    pub fn handle_input(&mut self, input: Event) -> io::Result<()> {
        // handle ctrlc
        if let Event::Key(key) = input {
            if let KeyCode::Char('c') = key.code {
                return Err(io::Error::new(io::ErrorKind::Interrupted, "Ctrl-C"));
            }
        }

        Ok(())
    }

    pub fn new_build_logs(&mut self, platform: Platform, update: UpdateBuildProgress) {
        let entry = self.build_logs.entry(platform).or_default();
        entry.update(update);
    }

    pub fn draw(
        &mut self,
        opts: &Serve,
        config: &DioxusCrate,
        build_engine: &Builder,
        server: &Server,
        watcher: &Watcher,
    ) {
        // just drain the build logs
        if !self.is_tty {
            return;
        }

        _ = self.term.draw(|frame| {
            // a layout that has a title with stats about the program and then the actual console itself
            let body = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(1), Constraint::Min(0)].as_ref())
                .split(frame.size());

            let header = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Min(20), Constraint::Fill(1)].as_ref())
                .split(body[0]);

            // Render a border for the header
            frame.render_widget(Block::default().borders(Borders::TOP), body[1]);

            // Render the metadata
            frame.render_widget(
                Paragraph::new(format!(
                    "dx serve | rustc-{rust_version}{channel} | dx-v{dx_version}",
                    rust_version = self.rustc_version,
                    channel = if self.rustc_nightly { "-nightly" } else { "" },
                    dx_version = self.dx_version
                ))
                .left_aligned(),
                header[0],
            );

            // The primary header
            frame.render_widget(
                Paragraph::new(format!("[/:more]",)).right_aligned(),
                header[1],
            );

            // // Render a two-column layout
            // let chunks = Layout::default()
            //     .direction(Direction::Horizontal)
            //     .constraints([Constraint::Max(20), Constraint::Min(0)].as_ref())
            //     .split(chunks[1]);

            // // The left column is a list of commands that we can interact with
            // let commands = vec![
            //     "Commands",
            //     "  Console",
            //     "  Configure",
            //     "  Edit",
            //     "  Add dep",
            //     "  Simulator",
            //     "  Bundle",
            //     "  Deploy",
            //     "  Lookbook",
            //     "  HTML to RSX",
            //     "  Builds",
            //     "  Debug",
            //     "  Visualize",
            //     "  Lint/Check",
            //     "  Share",
            //     "  Shortcuts",
            //     "  Learn",
            //     "  Help",
            // ];

            // let commands = commands.iter().map(|c| Span::styled(*c, Style::default()));

            // let commands = List::new(commands)
            //     .block(Block::default().borders(Borders::ALL))
            //     .style(Style::default().fg(Color::White))
            //     .highlight_style(Style::default().add_modifier(Modifier::BOLD))
            //     .highlight_symbol("> ");

            // frame.render_stateful_widget(commands, chunks[0], &mut self.command_list);

            // // The right is the output of that command
            // let output = vec![
            //     "Output",
            //     "  Compiling dioxus v0.1.0 (/Users/kevin/Projects/dioxus)",
            //     "    Finished dev [unoptimized + debuginfo] target(s) in 0.23s",
            //     "  Running `target/debug/dioxus`",
            //     "    dx run -i | rust 1.70 | stable | dx 0.5.2
            //         ",
            // ];

            // let output = output.iter().map(|c| Span::styled(*c, Style::default()));

            // let output = List::new(output)
            //     .block(Block::default().borders(Borders::ALL))
            //     .style(Style::default().fg(Color::White))
            //     .highlight_style(Style::default().add_modifier(Modifier::BOLD))
            //     .highlight_symbol("> ");

            // frame.render_widget(output, chunks[1]);
        });

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

fn set_fix_term_hook() {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        _ = disable_raw_mode();
        _ = stdout().execute(LeaveAlternateScreen);
        original_hook(info);
    }));
}

async fn rustc_version() -> String {
    tokio::process::Command::new("rustc")
        .arg("--version")
        .output()
        .await
        .ok()
        .map(|o| o.stdout)
        .map(|o| {
            let out = String::from_utf8(o).unwrap();
            out.split_ascii_whitespace().nth(1).map(|v| v.to_string())
        })
        .flatten()
        .unwrap_or_else(|| "<unknown>".to_string())
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

#[derive(Default)]
pub struct ActiveBuild {
    stage: Stage,
    messages: Vec<BuildMessage>,
    progress: f64,
}

impl ActiveBuild {
    fn update(&mut self, update: UpdateBuildProgress) {
        match update.update {
            UpdateStage::Start => {
                self.stage = update.stage;
                self.progress = 0.0;
            }
            UpdateStage::AddMessage(message) => {
                self.messages.push(message);
            }
            UpdateStage::SetProgress(progress) => {
                self.progress = progress;
            }
        }
    }
}
