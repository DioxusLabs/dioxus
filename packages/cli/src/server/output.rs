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
    is_cli_release: bool,
    platform: Platform,
    fly_modal: FlyModal,
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

        dx_version.push_str(env!("CARGO_PKG_VERSION"));

        let is_cli_release = crate::dx_build_info::PROFILE == "release";

        if !is_cli_release {
            if let Some(hash) = crate::dx_build_info::GIT_COMMIT_HASH_SHORT {
                let hash = &hash.trim_start_matches("g")[..4];
                dx_version.push_str("-");
                dx_version.push_str(hash);
            }
        }

        let platform = cfg.build_arguments.platform.expect("To be resolved by now");

        let fly_modal = FlyModal::new();

        Ok(Self {
            term,
            events,
            command_list,
            rustc_version,
            rustc_nightly,
            dx_version,
            is_tty,
            is_cli_release,
            platform,
            fly_modal,
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
            for (platform, build) in self.build_logs.iter() {
                if build.messages.is_empty() {
                    println!("No build logs for {platform:?}");
                    continue;
                }
                println!("Build logs for {platform:?}:");
                for message in build.messages.iter() {
                    println!("[{}] {:?}", message.level, message.message);
                }
            }
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

        match input {
            Event::Key(key) => {
                if let KeyCode::Char('/') = key.code {
                    self.fly_modal.hidden = !self.fly_modal.hidden;
                }
            }
            _ => {}
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
                .constraints([Constraint::Length(2), Constraint::Min(0)].as_ref())
                .split(frame.size());

            let header = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Fill(1)].as_ref())
                .split(body[0]);

            // Render a border for the header
            frame.render_widget(Block::default().borders(Borders::BOTTOM), body[0]);

            // Render the metadata
            let mut spans = vec![
                Span::from(if self.is_cli_release { "dx" } else { "dx-dev" }).green(),
                Span::from(" ").green(),
                Span::from("serve").green(),
                Span::from(" | ").white(),
                Span::from("v").cyan(),
                Span::from(self.dx_version.clone()).cyan(),
                // Span::from(" | ").white(),
                // Span::from("rustc-").cyan(),
                // Span::from(self.rustc_version.clone()).cyan(),
                // Span::from(if self.rustc_nightly { "-nightly" } else { "" }).cyan(),
                Span::from(" | ").white(),
                Span::from(self.platform.to_string()).cyan(),
                Span::from(" | ").white(),
            ];

            for (cmd, name) in [("/", "more"), ("?", "help")].iter() {
                spans.extend_from_slice(&[
                    Span::from("[").magenta(),
                    Span::from(*cmd).white(),
                    Span::from(" ").magenta(),
                    Span::from(*name).gray(),
                    Span::from("] ").magenta(),
                ]);
            }

            frame.render_widget(Paragraph::new(Line::from(spans)).left_aligned(), header[0]);

            for platform in self.build_logs.keys() {
                let build = self.build_logs.get(platform).unwrap();

                // Render the build logs with the last N lines where N is the height of the body
                let spans_to_take = body[1].height as usize;

                let mut spans = vec![Span::from("Build logs:")];

                let spans_iter = build.messages.iter().rev().take(spans_to_take).rev();

                for span in spans_iter {
                    spans.extend_from_slice(&[
                        Span::from("[").magenta(),
                        Span::from(span.level.to_string()).white(),
                        Span::from("] ").magenta(),
                        Span::from(format!("{:#?}", span.message)).gray(),
                        Span::from("\n").magenta(),
                    ]);
                }

                let paragraph = Paragraph::new(Line::from(spans)).left_aligned();
                frame.render_widget(paragraph, body[1]);
            }

            // render the fly modal
            self.fly_modal.render(frame, body[1]);
        });
    }
}

struct FlyModal {
    hidden: bool,
}

impl FlyModal {
    fn new() -> Self {
        Self { hidden: true }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        if self.hidden {
            return;
        }

        // Create a frame slightly smaller than the area
        let panel = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Fill(1)].as_ref())
            .margin(2)
            .split(area)[0];

        // Wipe the panel
        frame.render_widget(Clear, panel);
        frame.render_widget(Block::default().borders(Borders::ALL), panel);

        let modal = Paragraph::new(
            "Hello world!\nHello world!\nHello world!\nHello world!\nHello world!\n",
        )
        .alignment(Alignment::Center);
        frame.render_widget(modal, panel);
    }
}

trait TuiTab {}

struct BuildOutputTab {}
struct PlatformLogsTab {}

#[derive(Default, Debug)]
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
