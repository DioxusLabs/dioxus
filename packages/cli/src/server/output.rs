use crate::{
    builder::{BuildMessage, MessageType, Stage, UpdateBuildProgress},
    dioxus_crate::DioxusCrate,
};
use crate::{
    builder::{BuildResult, UpdateStage},
    serve::Serve,
};
use crossterm::{
    event::{Event, EventStream, KeyCode, KeyModifiers, MouseEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    tty::IsTty,
    ExecutableCommand,
};
use dioxus_cli_config::Platform;
use futures_util::{
    future::{join_all, FutureExt},
    StreamExt,
};
use ratatui::{prelude::*, widgets::*, TerminalOptions, Viewport};
use std::{
    cell::RefCell,
    collections::HashMap,
    io::{self, stdout},
    rc::Rc,
    time::{Duration, Instant},
};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, BufReader, Lines},
    process::{ChildStderr, ChildStdout},
};
use tracing::Level;

use super::{Builder, Server, Watcher};

pub struct Output {
    term: Rc<RefCell<TerminalBackend>>,
    events: EventStream,
    command_list: ListState,
    rustc_version: String,
    rustc_nightly: bool,
    dx_version: String,
    is_tty: bool,
    build_logs: HashMap<Platform, ActiveBuild>,
    running_apps: HashMap<Platform, RunningApp>,
    is_cli_release: bool,
    platform: Platform,

    num_lines_with_wrapping: u16,
    term_height: u16,
    scroll: u16,
    fly_modal_open: bool,
    anim_start: Instant,
}

enum StatusLine {
    Compiling,
    Hotreloading,
    Hotreloaded,
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
                dx_version.push('-');
                dx_version.push_str(hash);
            }
        }

        let platform = cfg.build_arguments.platform.expect("To be resolved by now");

        Ok(Self {
            term: Rc::new(RefCell::new(term)),
            events,
            command_list,
            rustc_version,
            rustc_nightly,
            dx_version,
            is_tty,
            is_cli_release,
            platform,
            fly_modal_open: false,
            build_logs: HashMap::new(),
            running_apps: HashMap::new(),
            scroll: 0,
            term_height: 0,
            num_lines_with_wrapping: 0,
            anim_start: Instant::now(),
        })
    }

    /// Wait for either the ctrl_c handler or the next event
    ///
    /// Why is the ctrl_c handler here?
    ///
    /// Also tick animations every few ms
    pub async fn wait(&mut self) -> io::Result<()> {
        let user_input = self.events.next();

        let next_stdout = self.running_apps.values_mut().map(|app| async move {
            let stdout = app.stdout.next_line();
            let stderr = app.stderr.next_line();
            tokio::select! {
                Ok(Some(line)) = stdout => (app.result.platform, Some(line), None),
                Ok(Some(line)) = stderr => (app.result.platform, None, Some(line)),
                _ = futures_util::future::pending() => (app.result.platform, None, None),
            }
        });

        let animation_timeout = tokio::time::sleep(Duration::from_millis(30));

        tokio::select! {
            new_line = join_all(next_stdout) => {
                for (platform, stdout, stderr) in new_line {
                    if let Some(stdout) = stdout {
                        self.running_apps.get_mut(&platform).unwrap().stdout_line.push_str(&stdout);
                        self.push_log(platform, BuildMessage {
                            level: Level::INFO,
                            message: MessageType::Text(stdout),
                        })
                    }
                    if let Some(stderr) = stderr {
                        self.running_apps.get_mut(&platform).unwrap().stderr_line.push_str(&stderr);
                        self.push_log(platform, BuildMessage {
                            level: Level::ERROR,
                            message: MessageType::Text(stderr),
                        })
                    }
                }
            },

            event = user_input => {
                self.handle_input(event.unwrap().unwrap())?;
            }

            _ = animation_timeout => {
            }
        }

        return Ok(());
    }

    pub fn shutdown(&mut self) -> io::Result<()> {
        // if we're a tty then we need to disable the raw mode
        if self.is_tty {
            disable_raw_mode()?;
            stdout().execute(LeaveAlternateScreen)?;

            // todo: print the build info here for the most recent build, and then the logs of the most recent build
            for (platform, build) in self.build_logs.iter() {
                if build.messages.is_empty() {
                    continue;
                }

                for message in build.messages.iter() {
                    match &message.message {
                        MessageType::Cargo(t) => {
                            println!("{}", t.rendered.as_deref().unwrap_or_default())
                        }
                        MessageType::Text(t) => println!("{}", t),
                    }
                }
            }
        }

        Ok(())
    }

    pub fn handle_input(&mut self, input: Event) -> io::Result<()> {
        // handle ctrlc
        if let Event::Key(key) = input {
            if let KeyCode::Char('c') = key.code {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    return Err(io::Error::new(io::ErrorKind::Interrupted, "Ctrl-C"));
                }
            }
        }

        if let Event::Key(key) = input {
            if let KeyCode::Char('/') = key.code {
                self.fly_modal_open = !self.fly_modal_open;
            }
        }

        match input {
            Event::Mouse(mouse) if mouse.kind == MouseEventKind::ScrollUp => {
                self.scroll = self.scroll.saturating_sub(1);
            }
            Event::Mouse(mouse) if mouse.kind == MouseEventKind::ScrollDown => {
                self.scroll += 1;
            }
            Event::Key(key) if key.code == KeyCode::Up => {
                self.scroll = self.scroll.saturating_sub(1);
            }
            Event::Key(key) if key.code == KeyCode::Down => {
                self.scroll += 1;
            }
            Event::Resize(_width, _height) => {
                // nothing, it should take care of itself
            }
            _ => {}
        }

        Ok(())
    }

    pub fn push_log(&mut self, platform: Platform, message: BuildMessage) {
        let snapped = self.is_snapped(platform);

        self.build_logs
            .get_mut(&platform)
            .unwrap()
            .messages
            .push(message);

        // // let log = self.build_logs.get(a).unwrap();
        // if snapped {
        //     self.scroll = (self.num_lines_with_wrapping).saturating_sub(self.term_height);
        //     // self.scroll = self.scroll.clamp(
        //     //     0,
        //     //     (self.num_lines_with_wrapping).saturating_sub(self.term_height),
        //     // ) as u16;
        //     //     self.scroll = self
        //     //         .num_lines_with_wrapping
        //     //         .saturating_sub(self.term_height) as u16;
        //     //     // self.scroll = log.messages.len().saturating_sub(self.term_height as usize) as u16;
        // }
    }

    fn is_snapped(&self, platform: Platform) -> bool {
        let prev_scrol = self
            .num_lines_with_wrapping
            .saturating_sub(self.term_height) as u16;
        prev_scrol == self.scroll
    }

    pub fn new_build_logs(&mut self, platform: Platform, update: UpdateBuildProgress) {
        let snapped = self.is_snapped(platform);

        self.build_logs.entry(platform).or_default().update(update);

        let log = self.build_logs.get(&platform).unwrap();
        if snapped {
            self.scroll = (self.num_lines_with_wrapping).saturating_sub(self.term_height);
            // self.scroll = self.scroll.clamp(
            //     0,
            //     (self.num_lines_with_wrapping).saturating_sub(self.term_height),
            // ) as u16;
            // self.scroll = self
            //     .num_lines_with_wrapping
            //     .saturating_sub(self.term_height) as u16;
            // self.scroll = log.messages.len().saturating_sub(self.term_height as usize) as u16;
        }
    }

    pub fn new_ready_app(&mut self, build_engine: &mut Builder, results: Vec<BuildResult>) {
        for result in results {
            let (stdout, stderr) = build_engine
                .children
                .iter_mut()
                .find_map(|(platform, child)| {
                    if platform == &result.platform {
                        Some((child.stdout.take().unwrap(), child.stderr.take().unwrap()))
                    } else {
                        None
                    }
                })
                .unwrap();

            let platform = result.platform.clone();

            let app = RunningApp {
                result,
                stdout: BufReader::new(stdout).lines(),
                stderr: BufReader::new(stderr).lines(),
                stdout_line: String::new(),
                stderr_line: String::new(),
            };

            self.running_apps.insert(platform, app);
        }
    }

    pub fn render(
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

        // Keep the animation track in terms of 100ms frames - the frame should be a number between 0 and 10
        // todo: we want to use this somehow to animate things...
        let elapsed = self.anim_start.elapsed().as_millis() as f32;
        let num_frames = elapsed / 100.0;
        let _frame_step = (num_frames % 10.0) as usize;

        _ = self.term.clone().borrow_mut().draw(|frame| {
            // a layout that has a title with stats about the program and then the actual console itself
            let body = Layout::default()
                .direction(Direction::Vertical)
                .constraints(
                    [
                        Constraint::Length(2),
                        Constraint::Min(0),
                        Constraint::Length(1),
                    ]
                    .as_ref(),
                )
                .split(frame.size());

            let header = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Fill(1)].as_ref())
                .split(body[0]);

            // Render a border for the header
            frame.render_widget(Block::default().borders(Borders::BOTTOM), body[0]);

            for platform in self.build_logs.keys() {
                let build = self.build_logs.get(platform).unwrap();

                // We're going to assemble a text buffer directly and then let the paragraph widgets
                // handle the wrapping and scrolling
                let mut paragraph_text: Text<'_> = Text::default();

                for span in build.messages.iter() {
                    use ansi_to_tui::IntoText;
                    match &span.message {
                        MessageType::Text(line) => {
                            paragraph_text.extend(line.as_str().into_text().unwrap_or_default());
                        }
                        MessageType::Cargo(diagnostic) => {
                            let diagnostic = diagnostic.rendered.as_deref().unwrap_or_default();
                            for line in diagnostic.lines() {
                                paragraph_text.extend(line.into_text().unwrap_or_default());
                            }
                        }
                    };
                }

                let paragraph = Paragraph::new(paragraph_text)
                    .left_aligned()
                    .wrap(Wrap { trim: false });

                self.term_height = body[1].height;
                self.num_lines_with_wrapping = paragraph.line_count(body[1].width) as u16;

                // if self.is_snapped(platform.clone()) {
                self.scroll = (self.num_lines_with_wrapping).saturating_sub(self.term_height);
                // }

                let paragraph = paragraph.scroll((self.scroll, 0));

                frame.render_widget(paragraph, body[1]);
            }

            // Render the metadata
            let mut spans = vec![
                Span::from(if self.is_cli_release { "dx" } else { "dx-dev" }).green(),
                Span::from(" ").green(),
                Span::from("serve").green(),
                Span::from(" | ").white(),
                // Span::from(frame_step.to_string()).cyan(),
                Span::from("v").cyan(),
                Span::from(self.dx_version.clone()).cyan(),
                // Span::from(" | ").white(),
                // Span::from("rustc-").cyan(),
                // Span::from(self.rustc_version.clone()).cyan(),
                // Span::from(if self.rustc_nightly { "-nightly" } else { "" }).cyan(),
                Span::from(" | ").white(),
                Span::from(self.platform.to_string()).cyan(),
                Span::from(" | ").white(),
                Span::from(self.scroll.to_string()).cyan(),
                Span::from("/").white(),
                Span::from(
                    (self
                        .num_lines_with_wrapping
                        .saturating_sub(self.term_height))
                    .to_string(),
                )
                .cyan(),
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

            // render the fly modal
            self.render_fly_moydal(frame, body[1], opts, config, build_engine, server, watcher);
        });
    }

    fn render_fly_moydal(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        opts: &Serve,
        config: &DioxusCrate,
        build_engine: &Builder,
        server: &Server,
        watcher: &Watcher,
    ) {
        if !self.fly_modal_open {
            return;
        }

        // Create a frame slightly smaller than the area
        let panel = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Fill(1)].as_ref())
            // .margin(2)
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
        .and_then(|o| {
            let out = String::from_utf8(o).unwrap();
            out.split_ascii_whitespace().nth(1).map(|v| v.to_string())
        })
        .unwrap_or_else(|| "<unknown>".to_string())
}

pub struct RunningApp {
    result: BuildResult,
    stdout: Lines<BufReader<ChildStdout>>,
    stderr: Lines<BufReader<ChildStderr>>,
    stdout_line: String,
    stderr_line: String,
}
