use crate::dioxus_crate::DioxusCrate;
use crate::serve::Serve;
use crossterm::{
    event::{self, Event, EventStream, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use futures_channel::mpsc::UnboundedReceiver;
use futures_util::{future::FutureExt, select, StreamExt};
use ratatui::{prelude::*, widgets::*, TerminalOptions, Viewport};
use std::io::{self, stdout};

use super::{Builder, Server, Watcher};

pub struct Output {
    term: TerminalBackend,
    events: EventStream,
    command_list: ListState,
    rustc_version: String,
}

type TerminalBackend = Terminal<CrosstermBackend<io::Stdout>>;

impl Output {
    pub async fn start(cfg: &Serve, crate_config: &DioxusCrate) -> io::Result<Self> {
        enable_raw_mode()?;
        stdout().execute(EnterAlternateScreen)?;
        let events = EventStream::new();
        let term: TerminalBackend = Terminal::with_options(
            CrosstermBackend::new(stdout()),
            TerminalOptions {
                viewport: Viewport::Fullscreen,
            },
        )?;

        let command_list = ListState::default();

        // get the rustc version
        let rustc_version = rustc_version().await;

        Ok(Self {
            term,
            events,
            command_list,
            rustc_version,
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
        disable_raw_mode()?;
        stdout().execute(LeaveAlternateScreen)?;
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

    pub fn draw(
        &mut self,
        opts: &Serve,
        config: &DioxusCrate,
        build_engine: &Builder,
        server: &Server,
        watcher: &Watcher,
    ) {
        _ = self.term.draw(|frame| {
            // a layout that has a title with stats about the program and then the actual console itself
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                // .margin(1)
                .constraints([Constraint::Length(0), Constraint::Min(0)].as_ref())
                .split(frame.size());

            // Render just a paragraph into the top chunks
            frame.render_widget(
                Paragraph::new(format!(
                    "dx serve | rust {version} | stable | dx 0.5.2",
                    version = self.rustc_version
                )),
                chunks[0],
            );

            // Render a two-column layout
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Max(20), Constraint::Min(0)].as_ref())
                .split(chunks[1]);

            // The left column is a list of commands that we can interact with
            let commands = vec![
                "Commands",
                "  Console",
                "  Configure",
                "  Edit",
                "  Add dep",
                "  Simulator",
                "  Bundle",
                "  Deploy",
                "  Lookbook",
                "  HTML to RSX",
                "  Builds",
                "  Debug",
                "  Visualize",
                "  Lint/Check",
                "  Share",
                "  Shortcuts",
                "  Learn",
                "  Help",
            ];

            let commands = commands.iter().map(|c| Span::styled(*c, Style::default()));

            let commands = List::new(commands)
                .block(Block::default().borders(Borders::ALL))
                .style(Style::default().fg(Color::White))
                .highlight_style(Style::default().add_modifier(Modifier::BOLD))
                .highlight_symbol("> ");

            frame.render_stateful_widget(commands, chunks[0], &mut self.command_list);

            // The right is the output of that command
            let output = vec![
                "Output",
                "  Compiling dioxus v0.1.0 (/Users/kevin/Projects/dioxus)",
                "    Finished dev [unoptimized + debuginfo] target(s) in 0.23s",
                "  Running `target/debug/dioxus`",
                "    dx run -i | rust 1.70 | stable | dx 0.5.2
                    ",
            ];

            let output = output.iter().map(|c| Span::styled(*c, Style::default()));

            let output = List::new(output)
                .block(Block::default().borders(Borders::ALL))
                .style(Style::default().fg(Color::White))
                .highlight_style(Style::default().add_modifier(Modifier::BOLD))
                .highlight_symbol("> ");

            frame.render_widget(output, chunks[1]);
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

async fn rustc_version() -> String {
    tokio::process::Command::new("rustc")
        .arg("--version")
        .output()
        .await
        .ok()
        .map(|o| o.stdout)
        .map(|o| {
            let out = String::from_utf8(o).unwrap();
            out.split_ascii_whitespace().nth(2).map(|v| v.to_string())
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
