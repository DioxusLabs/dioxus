use crate::{
    builder::BuildRequest,
    config::{AddressArguments, Platform},
};
use crate::{builder::UpdateStage, serve::Serve};
use crate::{
    builder::{
        BuildMessage, MessageSource, MessageType, Stage, TargetPlatform, UpdateBuildProgress,
    },
    dioxus_crate::DioxusCrate,
    serve::next_or_pending,
    tracer::CLILogControl,
};
use core::panic;
use crossterm::{
    event::{Event, EventStream, KeyCode, KeyModifiers, MouseEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    tty::IsTty,
    ExecutableCommand,
};
use dioxus_devtools_types::ClientMsg;
use futures_util::{future::select_all, Future, FutureExt, StreamExt};
use ratatui::{prelude::*, widgets::*, TerminalOptions, Viewport};
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    fmt::Display,
    io::{self, stdout},
    rc::Rc,
    sync::atomic::Ordering,
    time::{Duration, Instant},
};
use tokio::{
    io::{AsyncBufReadExt, BufReader, Lines},
    process::{ChildStderr, ChildStdout},
};
use tracing::Level;

use super::{update::ServeUpdate, Builder, DevServer, Watcher};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LogSource {
    Internal,
    Target(TargetPlatform),
}

impl Display for LogSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogSource::Internal => write!(f, "CLI"),
            LogSource::Target(platform) => write!(f, "{platform}"),
        }
    }
}

impl From<TargetPlatform> for LogSource {
    fn from(platform: TargetPlatform) -> Self {
        LogSource::Target(platform)
    }
}

#[derive(Default)]
pub struct BuildProgress {
    internal_logs: Vec<BuildMessage>,
    build_logs: HashMap<TargetPlatform, ActiveBuild>,
}

impl BuildProgress {
    pub fn progress(&self) -> f64 {
        self.build_logs
            .values()
            .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .map(|build| match build.stage {
                Stage::Initializing => 0.0,
                Stage::InstallingWasmTooling => 0.0,
                Stage::Compiling => build.progress,
                Stage::OptimizingWasm | Stage::OptimizingAssets | Stage::Finished => 1.0,
            })
            .unwrap_or_default()
    }
}

pub struct Output {
    term: Rc<RefCell<Option<TerminalBackend>>>,
    log_control: CLILogControl,

    // optional since when there's no tty there's no eventstream to read from - just stdin
    events: Option<EventStream>,

    _rustc_version: String,
    _rustc_nightly: bool,
    _dx_version: String,
    interactive: bool,
    pub(crate) build_progress: BuildProgress,
    is_cli_release: bool,
    platform: Platform,

    num_lines_with_wrapping: u16,
    term_height: u16,
    scroll: u16,
    fly_modal_open: bool,
    anim_start: Instant,

    tab: Tab,

    addr: AddressArguments,
}

#[derive(PartialEq, Eq, Clone, Copy)]
enum Tab {
    Console,
    BuildLog,
}

type TerminalBackend = Terminal<CrosstermBackend<io::Stdout>>;

impl Output {
    pub fn start(cfg: &Serve, log_control: CLILogControl) -> io::Result<Self> {
        let interactive = std::io::stdout().is_tty() && cfg.interactive.unwrap_or(true);

        let mut events = None;

        if interactive {
            log_control.tui_enabled.store(true, Ordering::SeqCst);
            enable_raw_mode()?;
            stdout().execute(EnterAlternateScreen)?;

            // workaround for ci where the terminal is not fully initialized
            // this stupid bug
            // https://github.com/crossterm-rs/crossterm/issues/659
            events = Some(EventStream::new());
        };

        // set the panic hook to fix the terminal
        set_fix_term_hook();

        let term: Option<TerminalBackend> = Terminal::with_options(
            CrosstermBackend::new(stdout()),
            TerminalOptions {
                viewport: Viewport::Fullscreen,
            },
        )
        .ok();

        // todo: re-enable rustc version
        // let rustc_version = rustc_version().await;
        // let rustc_nightly = rustc_version.contains("nightly") || cfg.target_args.nightly;
        let _rustc_version = String::from("1.0.0");
        let _rustc_nightly = false;

        let mut dx_version = String::new();

        dx_version.push_str(env!("CARGO_PKG_VERSION"));

        let is_cli_release = crate::dx_build_info::PROFILE == "release";

        if !is_cli_release {
            if let Some(hash) = crate::dx_build_info::GIT_COMMIT_HASH_SHORT {
                let hash = &hash.trim_start_matches('g')[..4];
                dx_version.push('-');
                dx_version.push_str(hash);
            }
        }

        let platform = cfg.build_arguments.platform.expect("To be resolved by now");

        Ok(Self {
            term: Rc::new(RefCell::new(term)),
            log_control,
            events,
            _rustc_version,
            _rustc_nightly,
            _dx_version: dx_version,
            interactive,
            is_cli_release,
            platform,
            fly_modal_open: false,
            build_progress: Default::default(),
            scroll: 0,
            term_height: 0,
            num_lines_with_wrapping: 0,
            anim_start: Instant::now(),
            tab: Tab::BuildLog,
            addr: cfg.server_arguments.address.clone(),
        })
    }

    /// Add a message from stderr to the logs
    fn push_stderr(&mut self, platform: TargetPlatform, stderr: String) {
        todo!()
        // self.set_tab(Tab::BuildLog);

        // self.running_apps
        //     .get_mut(&platform)
        //     .unwrap()
        //     .output
        //     .as_mut()
        //     .unwrap()
        //     .stderr_line
        //     .push_str(&stderr);
        // self.build_progress
        //     .build_logs
        //     .get_mut(&platform)
        //     .unwrap()
        //     .messages
        //     .push(BuildMessage {
        //         level: Level::ERROR,
        //         message: MessageType::Text(stderr),
        //         source: MessageSource::App,
        //     });
    }

    /// Add a message from stdout to the logs
    fn push_stdout(&mut self, platform: TargetPlatform, stdout: String) {
        todo!()
        // self.running_apps
        //     .get_mut(&platform)
        //     .unwrap()
        //     .output
        //     .as_mut()
        //     .unwrap()
        //     .stdout_line
        //     .push_str(&stdout);
        // self.build_progress
        //     .build_logs
        //     .get_mut(&platform)
        //     .unwrap()
        //     .messages
        //     .push(BuildMessage {
        //         level: Level::INFO,
        //         message: MessageType::Text(stdout),
        //         source: MessageSource::App,
        //     });
    }

    /// Wait for either the ctrl_c handler or the next event
    ///
    /// Why is the ctrl_c handler here?
    ///
    /// Also tick animations every few ms
    pub async fn wait(&mut self) -> io::Result<ServeUpdate> {
        fn ok_and_some<F, T, E>(f: F) -> impl Future<Output = T>
        where
            F: Future<Output = Result<Option<T>, E>>,
        {
            next_or_pending(async move { f.await.ok().flatten() })
        }

        let user_input = async {
            let events = self.events.as_mut()?;
            events.next().await
        };

        let user_input = ok_and_some(user_input.map(|e| e.transpose()));

        // let has_running_apps = !self.running_apps.is_empty();
        // let next_stdout = self.running_apps.values_mut().map(|app| {
        //     let future = async move {
        //         let (stdout, stderr) = match &mut app.output {
        //             Some(out) => (
        //                 ok_and_some(out.stdout.next_line()),
        //                 ok_and_some(out.stderr.next_line()),
        //             ),
        //             None => return futures_util::future::pending().await,
        //         };

        //         tokio::select! {
        //             line = stdout => (app.result.target_platform, Some(line), None),
        //             line = stderr => (app.result.target_platform, None, Some(line)),
        //         }
        //     };
        //     Box::pin(future)
        // });

        // let next_stdout = async {
        //     if has_running_apps {
        //         select_all(next_stdout).await.0
        //     } else {
        //         futures_util::future::pending().await
        //     }
        // };

        // let tui_log_rx = &mut self.log_control.tui_rx;
        // let next_tui_log = next_or_pending(tui_log_rx.next());

        // tokio::select! {
        //     (platform, stdout, stderr) = next_stdout => {
        //         if let Some(stdout) = stdout {
        //             self.push_stdout(platform, stdout);
        //         }
        //         if let Some(stderr) = stderr {
        //             self.push_stderr(platform, stderr);
        //         }
        //     },

        //     // Handle internal CLI tracing logs.
        //     log = next_tui_log => {
        //         self.push_log(LogSource::Internal, BuildMessage {
        //             level: Level::INFO,
        //             message: MessageType::Text(log),
        //             source: MessageSource::Dev,
        //         });
        //     }

        //     event = user_input => {
        //         if self.handle_events(event).await? {
        //             return Ok(ServeUpdate::TuiInput { rebuild: true });
        //         }
        //     }
        // }

        Ok(ServeUpdate::TuiInput { rebuild: false })
    }

    pub fn shutdown(&mut self) -> io::Result<()> {
        // if we're a tty then we need to disable the raw mode
        if self.interactive {
            self.log_control.tui_enabled.store(false, Ordering::SeqCst);
            disable_raw_mode()?;
            stdout().execute(LeaveAlternateScreen)?;
            self.drain_print_logs();
        }

        Ok(())
    }

    /// Emit the build logs as println! statements such that the terminal has the same output as cargo
    ///
    /// This is used when the terminal is shutdown and we want the build logs in the terminal. Old
    /// versions of the cli would just eat build logs making debugging issues harder than they needed
    /// to be.
    fn drain_print_logs(&mut self) {
        fn log_build_message(platform: &LogSource, message: &BuildMessage) {
            match &message.message {
                MessageType::Text(text) => {
                    for line in text.lines() {
                        println!("{platform}: {line}");
                    }
                }
                MessageType::Cargo(diagnostic) => {
                    println!("{platform}: {diagnostic}");
                }
            }
        }

        // todo: print the build info here for the most recent build, and then the logs of the most recent build
        for (platform, build) in self.build_progress.build_logs.iter_mut() {
            if build.messages.is_empty() {
                continue;
            }

            let messages = build.messages.drain(0..);

            for message in messages {
                log_build_message(&LogSource::Target(*platform), &message);
            }
        }

        // Log the internal logs
        let messaegs = self.build_progress.internal_logs.drain(..);
        for message in messaegs {
            log_build_message(&LogSource::Internal, &message);
        }
    }

    /// Handle an input event, returning `true` if the event should cause the program to restart.
    pub fn handle_input(&mut self, input: Event) -> io::Result<bool> {
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
            Event::Key(key) if key.code == KeyCode::Char('r') => {
                // todo: reload the app
                return Ok(true);
            }
            Event::Key(key) if key.code == KeyCode::Char('o') => {
                // Open the running app.
                open::that(format!("http://{}:{}", self.addr.addr, self.addr.port))?;
            }
            Event::Key(key) if key.code == KeyCode::Char('c') => {
                // Clear the currently selected build logs.
                for build in self.build_progress.build_logs.values_mut() {
                    let msgs = match self.tab {
                        Tab::Console => &mut build.stdout_logs,
                        Tab::BuildLog => &mut build.messages,
                    };
                    msgs.clear();
                }
            }
            Event::Key(key) if key.code == KeyCode::Char('1') => self.set_tab(Tab::Console),
            Event::Key(key) if key.code == KeyCode::Char('2') => self.set_tab(Tab::BuildLog),
            Event::Resize(_width, _height) => {
                // nothing, it should take care of itself
            }
            _ => {}
        }

        if self.scroll
            > self
                .num_lines_with_wrapping
                .saturating_sub(self.term_height + 1)
        {
            self.scroll = self
                .num_lines_with_wrapping
                .saturating_sub(self.term_height + 1);
        }

        Ok(false)
    }

    pub fn new_ws_message(
        &mut self,
        platform: TargetPlatform,
        message: axum::extract::ws::Message,
    ) {
        if let axum::extract::ws::Message::Text(text) = message {
            let msg = serde_json::from_str::<ClientMsg>(text.as_str());
            match msg {
                Ok(ClientMsg::Log { level, messages }) => {
                    self.push_log(
                        platform,
                        BuildMessage {
                            level: match level.as_str() {
                                "info" => Level::INFO,
                                "warn" => Level::WARN,
                                "error" => Level::ERROR,
                                "debug" => Level::DEBUG,
                                _ => Level::INFO,
                            },
                            message: MessageType::Text(
                                // todo: the js console is giving us a list of params, not formatted text
                                // we need to translate its styling into our own
                                messages.first().unwrap_or(&String::new()).clone(),
                            ),
                            source: MessageSource::App,
                        },
                    );
                }
                Err(err) => {
                    self.push_log(
                        platform,
                        BuildMessage {
                            level: Level::ERROR,
                            source: MessageSource::Dev,
                            message: MessageType::Text(format!("Error parsing app message: {err}")),
                        },
                    );
                }
            }
        }
    }

    // todo: re-enable
    #[allow(unused)]
    fn is_snapped(&self, _platform: LogSource) -> bool {
        true
        // let prev_scrol = self
        //     .num_lines_with_wrapping
        //     .saturating_sub(self.term_height);
        // prev_scrol == self.scroll
    }

    pub fn scroll_to_bottom(&mut self) {
        self.scroll = (self.num_lines_with_wrapping).saturating_sub(self.term_height);
    }

    pub fn push_log(&mut self, platform: impl Into<LogSource>, message: BuildMessage) {
        let source = platform.into();
        let snapped = self.is_snapped(source);

        match source {
            LogSource::Internal => self.build_progress.internal_logs.push(message),
            LogSource::Target(platform) => self
                .build_progress
                .build_logs
                .entry(platform)
                .or_default()
                .stdout_logs
                .push(message),
        }

        if snapped {
            self.scroll_to_bottom();
        }
    }

    pub fn new_build_logs(&mut self, platform: TargetPlatform, update: UpdateBuildProgress) {
        let snapped = self.is_snapped(LogSource::Target(platform));

        // when the build is finished, switch to the console
        if update.stage == Stage::Finished {
            self.tab = Tab::Console;
        }

        self.build_progress
            .build_logs
            .entry(platform)
            .or_default()
            .update(update);

        if snapped {
            self.scroll_to_bottom();
        }
    }

    pub fn new_ready_app(&mut self, build_engine: &mut Builder, target: TargetPlatform) {
        todo!()
        // for result in results {
        //     let out = build_engine
        //         .finished
        //         .iter_mut()
        //         .find_map(|(platform, child)| {
        //             if platform == &result.target_platform {
        //                 let stdout = child.stdout.take().unwrap();
        //                 let stderr = child.stderr.take().unwrap();
        //                 Some((stdout, stderr))
        //             } else {
        //                 None
        //             }
        //         });

        //     let platform = result.target_platform;

        //     let stdout = out.map(|(stdout, stderr)| RunningAppOutput {
        //         stdout: BufReader::new(stdout).lines(),
        //         stderr: BufReader::new(stderr).lines(),
        //         stdout_line: String::new(),
        //         stderr_line: String::new(),
        //     });

        //     let app = RunningApp {
        //         result,
        //         output: stdout,
        //     };

        //     self.running_apps.insert(platform, app);

        //     // Finish the build progress for the platform that just finished building
        //     if let Some(build) = self.build_progress.build_logs.get_mut(&platform) {
        //         build.stage = Stage::Finished;
        //     }
        // }
    }

    pub fn render(
        &mut self,
        _opts: &Serve,
        _config: &DioxusCrate,
        _build_engine: &Builder,
        server: &DevServer,
        _watcher: &Watcher,
    ) {
        // just drain the build logs
        if !self.interactive {
            self.drain_print_logs();
            return;
        }

        // Keep the animation track in terms of 100ms frames - the frame should be a number between 0 and 10
        // todo: we want to use this somehow to animate things...
        let elapsed = self.anim_start.elapsed().as_millis() as f32;
        let num_frames = elapsed / 100.0;
        let _frame_step = (num_frames % 10.0) as usize;

        _ = self
            .term
            .clone()
            .borrow_mut()
            .as_mut()
            .unwrap()
            .draw(|frame| {
                // a layout that has a title with stats about the program and then the actual console itself
                let body = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints(
                        [
                            // Title
                            Constraint::Length(1),
                            // Body
                            Constraint::Min(0),
                        ]
                        .as_ref(),
                    )
                    .split(frame.size());

                // Split the body into a left and a right
                let console = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Fill(1), Constraint::Length(14)].as_ref())
                    .split(body[1]);

                let addr = format!("http://{}:{}", self.addr.addr, self.addr.port);
                let listening_len = format!("listening at {addr}").len() + 3;
                let listening_len = if listening_len > body[0].width as usize {
                    0
                } else {
                    listening_len
                };

                let header = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints(
                        [
                            Constraint::Fill(1),
                            Constraint::Length(listening_len as u16),
                        ]
                        .as_ref(),
                    )
                    .split(body[0]);

                // // Render a border for the header
                // frame.render_widget(Block::default().borders(Borders::BOTTOM), body[0]);

                // Render the metadata
                let mut spans: Vec<Span> = vec![
                    Span::from(if self.is_cli_release { "dx" } else { "dx-dev" }).green(),
                    Span::from(" ").green(),
                    Span::from("serve").green(),
                    Span::from(" | ").white(),
                    Span::from(self.platform.to_string()).green(),
                    Span::from(" | ").white(),
                ];

                // If there is build progress, display that next to the platform
                if !self.build_progress.build_logs.is_empty() {
                    if self
                        .build_progress
                        .build_logs
                        .values()
                        .any(|b| b.failed.is_some())
                    {
                        spans.push(Span::from("build failed ❌").red());
                    } else {
                        spans.push(Span::from("status: ").green());
                        let build = self
                            .build_progress
                            .build_logs
                            .values()
                            .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                            .unwrap();
                        spans.extend_from_slice(&build.spans(Rect::new(
                            0,
                            0,
                            build.max_layout_size(),
                            1,
                        )));
                    }
                }

                frame.render_widget(Paragraph::new(Line::from(spans)).left_aligned(), header[0]);

                // Split apart the body into a center and a right side
                // We only want to show the sidebar if there's enough space
                if listening_len > 0 {
                    frame.render_widget(
                        Paragraph::new(Line::from(vec![
                            Span::from("listening at ").dark_gray(),
                            Span::from(format!("http://{}", server.ip).as_str()).gray(),
                        ])),
                        header[1],
                    );
                }

                // Draw the tabs in the right region of the console
                // First draw the left border
                frame.render_widget(
                    Paragraph::new(vec![
                        {
                            let mut line = Line::from(" [1] console").dark_gray();
                            if self.tab == Tab::Console {
                                line.style = Style::default().fg(Color::LightYellow);
                            }
                            line
                        },
                        {
                            let mut line = Line::from(" [2] build").dark_gray();
                            if self.tab == Tab::BuildLog {
                                line.style = Style::default().fg(Color::LightYellow);
                            }
                            line
                        },
                        Line::from("  ").gray(),
                        Line::from(" [/] more").gray(),
                        Line::from(" [r] rebuild").gray(),
                        Line::from(" [c] clear").gray(),
                        Line::from(" [o] open").gray(),
                        Line::from(" [h] hide").gray(),
                    ])
                    .left_aligned()
                    .block(
                        Block::default()
                            .borders(Borders::LEFT | Borders::TOP)
                            .border_set(symbols::border::Set {
                                top_left: symbols::line::NORMAL.horizontal_down,
                                ..symbols::border::PLAIN
                            }),
                    ),
                    console[1],
                );

                // We're going to assemble a text buffer directly and then let the paragraph widgets
                // handle the wrapping and scrolling
                let mut paragraph_text: Text<'_> = Text::default();

                let mut add_build_message = |message: &BuildMessage| {
                    use ansi_to_tui::IntoText;
                    match &message.message {
                        MessageType::Text(line) => {
                            for line in line.lines() {
                                let text = line.into_text().unwrap_or_default();
                                for line in text.lines {
                                    let source = format!("[{}] ", message.source);

                                    let msg_span = Span::from(source);
                                    let msg_span = match message.source {
                                        MessageSource::App => msg_span.light_blue(),
                                        MessageSource::Dev => msg_span.dark_gray(),
                                        MessageSource::Build => msg_span.light_yellow(),
                                    };

                                    let mut out_line = vec![msg_span];
                                    for span in line.spans {
                                        out_line.push(span);
                                    }
                                    let newline = Line::from(out_line);
                                    paragraph_text.push_line(newline);
                                }
                            }
                        }
                        MessageType::Cargo(diagnostic) => {
                            let diagnostic = diagnostic.rendered.as_deref().unwrap_or_default();

                            for line in diagnostic.lines() {
                                paragraph_text.extend(line.into_text().unwrap_or_default());
                            }
                        }
                    };
                };

                // First log each platform's build logs
                for platform in self.build_progress.build_logs.keys() {
                    let build = self.build_progress.build_logs.get(platform).unwrap();

                    let msgs = match self.tab {
                        Tab::Console => &build.stdout_logs,
                        Tab::BuildLog => &build.messages,
                    };

                    for span in msgs.iter() {
                        add_build_message(span);
                    }
                }
                // Then log the internal logs
                for message in self.build_progress.internal_logs.iter() {
                    add_build_message(message);
                }

                let paragraph = Paragraph::new(paragraph_text)
                    .left_aligned()
                    .wrap(Wrap { trim: false });

                self.term_height = console[0].height;
                self.num_lines_with_wrapping = paragraph.line_count(console[0].width) as u16;

                let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                    .begin_symbol(None)
                    .end_symbol(None)
                    .track_symbol(None)
                    .thumb_symbol("▐");

                let mut scrollbar_state = ScrollbarState::new(
                    self.num_lines_with_wrapping
                        .saturating_sub(self.term_height) as usize,
                )
                .position(self.scroll as usize);

                let paragraph = paragraph.scroll((self.scroll, 0));
                paragraph
                    .block(Block::new().borders(Borders::TOP))
                    .render(console[0], frame.buffer_mut());

                // and the scrollbar, those are separate widgets
                frame.render_stateful_widget(
                    scrollbar,
                    console[0].inner(Margin {
                        // todo: dont use margin - just push down the body based on its top border
                        // using an inner vertical margin of 1 unit makes the scrollbar inside the block
                        vertical: 1,
                        horizontal: 0,
                    }),
                    &mut scrollbar_state,
                );

                // render the fly modal
                self.render_fly_modal(frame, console[0]);
            });
    }

    async fn handle_events(&mut self, event: Event) -> io::Result<bool> {
        let mut events = vec![event];

        // Collect all the events within the next 10ms in one stream
        let collect_events = async {
            loop {
                let Some(Ok(next)) = self.events.as_mut().unwrap().next().await else {
                    break;
                };
                events.push(next);
            }
        };
        tokio::select! {
            _ = collect_events => {},
            _ = tokio::time::sleep(Duration::from_millis(10)) => {}
        }

        // Debounce events within the same frame
        let mut handled = HashSet::new();
        for event in events {
            if !handled.contains(&event) {
                if self.handle_input(event.clone())? {
                    // Restart the running app.
                    return Ok(true);
                }
                handled.insert(event);
            }
        }

        Ok(false)
    }

    fn render_fly_modal(&mut self, frame: &mut Frame, area: Rect) {
        if !self.fly_modal_open {
            return;
        }

        // Create a frame slightly smaller than the area
        let panel = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Fill(1)].as_ref())
            .split(area)[0];

        // Wipe the panel
        frame.render_widget(Clear, panel);
        frame.render_widget(Block::default().borders(Borders::ALL), panel);

        let modal = Paragraph::new("Under construction, please check back at a later date!\n")
            .alignment(Alignment::Center);
        frame.render_widget(modal, panel);
    }

    fn set_tab(&mut self, tab: Tab) {
        self.tab = tab;
        self.scroll = 0;
    }
}

#[derive(Default, Debug, PartialEq)]
pub struct ActiveBuild {
    stage: Stage,
    messages: Vec<BuildMessage>,
    stdout_logs: Vec<BuildMessage>,
    progress: f64,
    failed: Option<String>,
}

impl ActiveBuild {
    fn update(&mut self, update: UpdateBuildProgress) {
        match update.update {
            UpdateStage::Start => {
                // If we are already past the stage, don't roll back, but allow a fresh build to update.
                if self.stage > update.stage && self.stage < Stage::Finished {
                    return;
                }
                self.stage = update.stage;
                self.progress = 0.0;
                self.failed = None;
            }
            UpdateStage::AddMessage(message) => {
                self.messages.push(message);
            }
            UpdateStage::SetProgress(progress) => {
                self.progress = progress;
            }
            UpdateStage::Failed(failed) => {
                self.stage = Stage::Finished;
                self.failed = Some(failed.clone());
            }
        }
    }

    fn spans(&self, area: Rect) -> Vec<Span> {
        let mut spans = Vec::new();

        let message = match self.stage {
            Stage::Initializing => "initializing... ",
            Stage::InstallingWasmTooling => "installing wasm tools... ",
            Stage::Compiling => "compiling... ",
            Stage::OptimizingWasm => "optimizing wasm... ",
            Stage::OptimizingAssets => "optimizing assets... ",
            Stage::Finished => "finished! 🎉 ",
        };
        let progress = format!("{}%", (self.progress * 100.0) as u8);

        if area.width >= self.max_layout_size() {
            spans.push(Span::from(message).light_yellow());

            if self.stage != Stage::Finished {
                spans.push(Span::from(progress).white());
            }
        } else {
            spans.push(Span::from(progress).white());
        }

        spans
    }

    fn max_layout_size(&self) -> u16 {
        let progress_size = 4;
        let stage_size = self.stage.to_string().len() as u16;
        let brace_size = 2;

        progress_size + stage_size + brace_size
    }
}

impl PartialOrd for ActiveBuild {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(
            self.stage
                .cmp(&other.stage)
                .then(self.progress.partial_cmp(&other.progress).unwrap()),
        )
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

// todo: re-enable
#[allow(unused)]
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
