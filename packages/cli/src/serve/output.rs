use crate::{
    builder::{BuildMessage, MessageType, Stage, UpdateBuildProgress},
    dioxus_crate::DioxusCrate,
};
use crate::{
    builder::{BuildResult, UpdateStage},
    serve::Serve,
};
use core::panic;
use crossterm::{
    event::{Event, EventStream, KeyCode, KeyModifiers, MouseEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    tty::IsTty,
    ExecutableCommand,
};
use dioxus_cli_config::Platform;
use dioxus_hot_reload::ClientMsg;
use futures_util::{future::select_all, Future, StreamExt};
use ratatui::{prelude::*, widgets::*, TerminalOptions, Viewport};
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    io::{self, stdout},
    pin::Pin,
    rc::Rc,
    time::{Duration, Instant},
};
use tokio::{
    io::{AsyncBufReadExt, BufReader, Lines},
    process::{ChildStderr, ChildStdout},
};
use tracing::Level;

use super::{Builder, Server, Watcher};

#[derive(Default)]
pub struct BuildProgress {
    build_logs: HashMap<Platform, ActiveBuild>,
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

    // optional since when there's no tty there's no eventstream to read from - just stdin
    events: Option<EventStream>,

    _rustc_version: String,
    _rustc_nightly: bool,
    _dx_version: String,
    interactive: bool,
    pub(crate) build_progress: BuildProgress,
    running_apps: HashMap<Platform, RunningApp>,
    is_cli_release: bool,
    platform: Platform,

    num_lines_with_wrapping: u16,
    term_height: u16,
    scroll: u16,
    fly_modal_open: bool,
    anim_start: Instant,

    tab: Tab,
}

#[derive(PartialEq, Eq, Clone, Copy)]
enum Tab {
    Console,
    BuildLog,
}

type TerminalBackend = Terminal<CrosstermBackend<io::Stdout>>;

impl Output {
    pub fn start(cfg: &Serve) -> io::Result<Self> {
        let interactive = std::io::stdout().is_tty() && cfg.interactive.unwrap_or(true);

        let mut events = None;

        if interactive {
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
            events,
            _rustc_version,
            _rustc_nightly,
            _dx_version: dx_version,
            interactive,
            is_cli_release,
            platform,
            fly_modal_open: false,
            build_progress: Default::default(),
            running_apps: HashMap::new(),
            scroll: 0,
            term_height: 0,
            num_lines_with_wrapping: 0,
            anim_start: Instant::now(),
            tab: Tab::BuildLog,
        })
    }

    /// Wait for either the ctrl_c handler or the next event
    ///
    /// Why is the ctrl_c handler here?
    ///
    /// Also tick animations every few ms
    pub async fn wait(&mut self) -> io::Result<()> {
        // sorry lord
        let user_input = match self.events.as_mut() {
            Some(events) => {
                let pinned: Pin<Box<dyn Future<Output = Option<Result<Event, _>>>>> =
                    Box::pin(events.next());
                pinned
            }
            None => Box::pin(futures_util::future::pending()) as Pin<Box<dyn Future<Output = _>>>,
        };

        let has_running_apps = !self.running_apps.is_empty();
        let next_stdout = self.running_apps.values_mut().map(|app| {
            let future = async move {
                let (stdout, stderr) = match &mut app.stdout {
                    Some(stdout) => (stdout.stdout.next_line(), stdout.stderr.next_line()),
                    None => return futures_util::future::pending().await,
                };

                tokio::select! {
                    Ok(Some(line)) = stdout => (app.result.platform, Some(line), None),
                    Ok(Some(line)) = stderr => (app.result.platform, None, Some(line)),
                    else => futures_util::future::pending().await,
                }
            };
            Box::pin(future)
        });

        let next_stdout = async {
            if has_running_apps {
                select_all(next_stdout).await.0
            } else {
                futures_util::future::pending().await
            }
        };

        let animation_timeout = tokio::time::sleep(Duration::from_millis(300));

        tokio::select! {
            (platform, stdout, stderr) = next_stdout => {
                if let Some(stdout) = stdout {
                    self.running_apps.get_mut(&platform).unwrap().stdout.as_mut().unwrap().stdout_line.push_str(&stdout);
                    self.push_log(platform, BuildMessage {
                        level: Level::INFO,
                        message: MessageType::Text(stdout),
                        source: Some("app".to_string()),
                    })
                }
                if let Some(stderr) = stderr {
                    self.running_apps.get_mut(&platform).unwrap().stdout.as_mut().unwrap().stderr_line.push_str(&stderr);
                    self.push_log(platform, BuildMessage {
                        level: Level::ERROR,
                        message: MessageType::Text(stderr),
                        source: Some("app".to_string()),
                    })
                }
            },

            event = user_input => {
                self.handle_events(event.unwrap().unwrap()).await?;
                // self.handle_input(event.unwrap().unwrap())?;
            }

            _ = animation_timeout => {}
        }

        Ok(())
    }

    pub fn shutdown(&mut self) -> io::Result<()> {
        // if we're a tty then we need to disable the raw mode
        if self.interactive {
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
        // todo: print the build info here for the most recent build, and then the logs of the most recent build
        for (platform, build) in self.build_progress.build_logs.iter_mut() {
            if build.messages.is_empty() {
                continue;
            }

            let messages = build.messages.drain(0..);

            for message in messages {
                match &message.message {
                    MessageType::Cargo(diagnostic) => {
                        println!(
                            "{platform}: {}",
                            diagnostic.rendered.as_deref().unwrap_or_default()
                        )
                    }
                    MessageType::Text(t) => println!("{platform}: {t}"),
                }
            }
        }
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
            Event::Key(key) if key.code == KeyCode::Char('r') => {}
            Event::Key(key) if key.code == KeyCode::Char('o') => {
                // todo: open the app
            }
            Event::Key(key) if key.code == KeyCode::Char('c') => {
                // clear
            }
            Event::Key(key) if key.code == KeyCode::Char('0') => {
                self.tab = Tab::Console;
                self.scroll = 0;
            }
            Event::Key(key) if key.code == KeyCode::Char('1') => {
                self.tab = Tab::BuildLog;
                self.scroll = 0;
            }
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

        Ok(())
    }

    pub fn new_ws_message(&mut self, platform: Platform, message: axum::extract::ws::Message) {
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
                            source: Some("app".to_string()),
                        },
                    );
                }
                Err(err) => {
                    self.push_log(
                        platform,
                        BuildMessage {
                            level: Level::ERROR,
                            source: Some("app".to_string()),
                            message: MessageType::Text(format!("Error parsing message: {err}")),
                        },
                    );
                }
            }
        }
    }

    // todo: re-enable
    #[allow(unused)]
    fn is_snapped(&self, _platform: Platform) -> bool {
        true
        // let prev_scrol = self
        //     .num_lines_with_wrapping
        //     .saturating_sub(self.term_height);
        // prev_scrol == self.scroll
    }

    pub fn scroll_to_bottom(&mut self) {
        self.scroll = (self.num_lines_with_wrapping).saturating_sub(self.term_height);
    }

    pub fn push_log(&mut self, platform: Platform, message: BuildMessage) {
        let snapped = self.is_snapped(platform);

        if let Some(build) = self.build_progress.build_logs.get_mut(&platform) {
            build.stdout_logs.push(message);
        }

        if snapped {
            self.scroll_to_bottom();
        }
    }

    pub fn new_build_logs(&mut self, platform: Platform, update: UpdateBuildProgress) {
        let snapped = self.is_snapped(platform);

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

    pub fn new_ready_app(&mut self, build_engine: &mut Builder, results: Vec<BuildResult>) {
        for result in results {
            let out = build_engine
                .children
                .iter_mut()
                .find_map(|(platform, child)| {
                    if platform == &result.platform {
                        let stdout = child.stdout.take().unwrap();
                        let stderr = child.stderr.take().unwrap();
                        Some((stdout, stderr))
                    } else {
                        None
                    }
                });

            let platform = result.platform;

            let stdout = out.map(|(stdout, stderr)| RunningAppOutput {
                stdout: BufReader::new(stdout).lines(),
                stderr: BufReader::new(stderr).lines(),
                stdout_line: String::new(),
                stderr_line: String::new(),
            });

            let app = RunningApp { result, stdout };

            self.running_apps.insert(platform, app);

            // Finish the build progress for the platform that just finished building
            if let Some(build) = self.build_progress.build_logs.get_mut(&platform) {
                build.stage = Stage::Finished;
            }
        }
    }

    pub fn render(
        &mut self,
        _opts: &Serve,
        _config: &DioxusCrate,
        _build_engine: &Builder,
        server: &Server,
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

                let listening_len = "listening at http://127.0.0.1:8080".len() + 3;
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
                let mut spans = vec![
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
                            .min_by(|a, b| a.partial_cmp(b).unwrap())
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
                            let mut line = Line::from(" [0] console").dark_gray();
                            if self.tab == Tab::Console {
                                line.style = Style::default().fg(Color::LightYellow);
                            }
                            line
                        },
                        {
                            let mut line = Line::from(" [1] build").dark_gray();
                            if self.tab == Tab::BuildLog {
                                line.style = Style::default().fg(Color::LightYellow);
                            }
                            line
                        },
                        Line::from("  ").gray(),
                        Line::from(" [/] more").gray(),
                        Line::from(" [r] reload").gray(),
                        Line::from(" [r] clear").gray(),
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

                for platform in self.build_progress.build_logs.keys() {
                    let build = self.build_progress.build_logs.get(platform).unwrap();

                    let msgs = match self.tab {
                        Tab::Console => &build.stdout_logs,
                        Tab::BuildLog => &build.messages,
                    };

                    for span in msgs.iter() {
                        use ansi_to_tui::IntoText;
                        match &span.message {
                            MessageType::Text(line) => {
                                for line in line.lines() {
                                    let text = line.into_text().unwrap_or_default();
                                    for line in text.lines {
                                        let mut out_line = vec![Span::from("[app] ").dark_gray()];
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
                    }
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

    async fn handle_events(&mut self, event: Event) -> io::Result<()> {
        let mut events = vec![event];

        // Collect all the events within the next 10ms in one stream
        loop {
            let next = self.events.as_mut().unwrap().next();
            tokio::select! {
                msg = next => events.push(msg.unwrap().unwrap()),
                _ = tokio::time::sleep(Duration::from_millis(1)) => break
            }
        }

        // Debounce events within the same frame
        let mut handled = HashSet::new();
        for event in events {
            if !handled.contains(&event) {
                self.handle_input(event.clone())?;
                handled.insert(event);
            }
        }

        Ok(())
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

        let modal = Paragraph::new(
            "Hello world!\nHello world!\nHello world!\nHello world!\nHello world!\n",
        )
        .alignment(Alignment::Center);
        frame.render_widget(modal, panel);
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

pub struct RunningApp {
    result: BuildResult,
    stdout: Option<RunningAppOutput>,
}

struct RunningAppOutput {
    stdout: Lines<BufReader<ChildStdout>>,
    stderr: Lines<BufReader<ChildStderr>>,
    stdout_line: String,
    stderr_line: String,
}
