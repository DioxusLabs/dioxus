use super::{Builder, Server, Watcher};
use crate::{
    builder::{BuildProgressUpdate, Stage, TargetPlatform},
    dioxus_crate::DioxusCrate,
    serve::next_or_pending,
    tracer::CLILogControl,
};
use crate::{
    builder::{BuildResult, UpdateStage},
    serve::Serve,
};
use core::panic;
use crossterm::{
    cursor::{Hide, Show},
    event::{
        DisableMouseCapture, EnableMouseCapture, Event, EventStream, KeyCode, KeyEventKind,
        KeyModifiers, MouseButton, MouseEventKind,
    },
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    tty::IsTty,
    ExecutableCommand,
};
use dioxus_cli_config::{AddressArguments, Platform};
use dioxus_hot_reload::ClientMsg;
use futures_util::{future::select_all, Future, FutureExt, StreamExt};
use ratatui::{prelude::*, TerminalOptions, Viewport};
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
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

mod message;
mod render;
mod selection;

pub use message::*;

// How many lines should be scroll on each mouse scroll or arrow key input.
const SCROLL_SPEED: u16 = 1;
// Speed added to `SCROLL_SPEED` when the modifier key is held during scroll.
const SCROLL_MODIFIER: u16 = 4;
// Scroll modifier key.
const SCROLL_MODIFIER_KEY: KeyModifiers = KeyModifiers::SHIFT;

#[derive(Default)]
pub struct BuildProgress {
    current_builds: HashMap<TargetPlatform, ActiveBuild>,
}

impl BuildProgress {
    pub fn progress(&self) -> f64 {
        self.current_builds
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

    pub(crate) build_progress: BuildProgress,
    running_apps: HashMap<TargetPlatform, RunningApp>,

    // A list of all messages from build, dev, app, and more.
    messages: Vec<Message>,

    num_lines_wrapping: u16,
    scroll_position: u16,
    console_width: u16,
    console_height: u16,

    more_modal_open: bool,
    anim_start: Instant,

    interactive: bool,
    is_cli_release: bool,
    platform: Platform,
    addr: AddressArguments,

    drag_start: Option<(u16, u16)>,
    drag_end: Option<(u16, u16)>,
    selected_lines: Vec<String>,

    // Filters
    show_filter_menu: bool,
    filters: Vec<(String, bool)>,
    selected_filter_index: usize,
    filter_search_mode: bool,
    filter_search_input: Option<String>,

    _rustc_version: String,
    _rustc_nightly: bool,
    _dx_version: String,
}

type TerminalBackend = Terminal<CrosstermBackend<io::Stdout>>;

impl Output {
    pub fn start(cfg: &Serve, log_control: CLILogControl) -> io::Result<Self> {
        let interactive = std::io::stdout().is_tty() && cfg.interactive.unwrap_or(true);

        let mut events = None;

        if interactive {
            log_control.output_enabled.store(true, Ordering::SeqCst);
            enable_raw_mode()?;
            stdout()
                .execute(EnterAlternateScreen)?
                .execute(EnableMouseCapture)?
                .execute(Hide)?;

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
            messages: Vec::new(),
            more_modal_open: false,
            build_progress: Default::default(),
            running_apps: HashMap::new(),
            scroll_position: 0,
            num_lines_wrapping: 0,
            console_width: 0,
            console_height: 0,
            anim_start: Instant::now(),
            addr: cfg.server_arguments.address.clone(),

            // Text selection
            drag_start: None,
            drag_end: None,
            selected_lines: Vec::new(),

            // Filter
            show_filter_menu: true,
            filters: Vec::new(),
            selected_filter_index: 0,
            filter_search_input: None,
            filter_search_mode: false,
        })
    }

    /// Add a message from stderr to the logs
    fn push_stderr(&mut self, platform: TargetPlatform, stderr: String) {
        self.running_apps
            .get_mut(&platform)
            .unwrap()
            .output
            .as_mut()
            .unwrap()
            .stderr_line
            .push_str(&stderr);

        self.messages.push(Message {
            source: MessageSource::App(platform),
            level: Level::ERROR,
            content: stderr,
        });
    }

    /// Add a message from stdout to the logs
    fn push_stdout(&mut self, platform: TargetPlatform, stdout: String) {
        self.running_apps
            .get_mut(&platform)
            .unwrap()
            .output
            .as_mut()
            .unwrap()
            .stdout_line
            .push_str(&stdout);

        self.messages.push(Message {
            source: MessageSource::App(platform),
            level: Level::INFO,
            content: stdout,
        });
    }

    /// Wait for either the ctrl_c handler or the next event
    ///
    /// Why is the ctrl_c handler here?
    ///
    /// Also tick animations every few ms
    pub async fn wait(&mut self) -> io::Result<bool> {
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

        let has_running_apps = !self.running_apps.is_empty();
        let next_stdout = self.running_apps.values_mut().map(|app| {
            let future = async move {
                let (stdout, stderr) = match &mut app.output {
                    Some(out) => (
                        ok_and_some(out.stdout.next_line()),
                        ok_and_some(out.stderr.next_line()),
                    ),
                    None => return futures_util::future::pending().await,
                };

                tokio::select! {
                    line = stdout => (app.result.target_platform, Some(line), None),
                    line = stderr => (app.result.target_platform, None, Some(line)),
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

        let tui_log_rx = &mut self.log_control.output_rx;
        let next_tui_log = next_or_pending(tui_log_rx.next());

        tokio::select! {
            (platform, stdout, stderr) = next_stdout => {
                if let Some(stdout) = stdout {
                    self.push_stdout(platform, stdout);
                }
                if let Some(stderr) = stderr {
                    self.push_stderr(platform, stderr);
                }
            },

            // Handle internal CLI tracing logs.
            log = next_tui_log => {
                self.push_log(log);
            }

            event = user_input => {
                if self.handle_events(event).await? {
                    return Ok(true)
                }
            }
        }

        Ok(false)
    }

    pub fn shutdown(&mut self) -> io::Result<()> {
        // if we're a tty then we need to disable the raw mode
        if self.interactive {
            self.log_control
                .output_enabled
                .store(false, Ordering::SeqCst);
            disable_raw_mode()?;
            stdout()
                .execute(DisableMouseCapture)?
                .execute(LeaveAlternateScreen)?
                .execute(Show)?;
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
        let messages = self.messages.drain(..);

        for msg in messages {
            // TODO: Better formatting for different content lengths.
            if msg.source != MessageSource::Cargo {
                println!("[{}] {}: {}", msg.source, msg.level, msg.content);
            } else {
                println!("{}", msg.content);
            }
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

        // If we're in filter search mode we must capture all key inputs.
        // This also handles when a filter is submitted.
        if self.filter_search_mode {
            if let Event::Key(key) = input {
                if key.kind != KeyEventKind::Press {
                    return Ok(false);
                }

                match key.code {
                    KeyCode::Char(c) => {
                        if let Some(input) = self.filter_search_input.as_mut() {
                            input.push(c);
                        } else {
                            self.filter_search_input = Some(String::from(c));
                        }
                    }
                    KeyCode::Enter => {
                        if let Some(search) = &self.filter_search_input {
                            self.filters.push((search.to_string(), true));
                        }
                        self.filter_search_input = None;
                        self.filter_search_mode = false;
                    }
                    KeyCode::Backspace => {
                        if let Some(search) = self.filter_search_input.as_mut() {
                            search.pop();
                            if search.is_empty() {
                                self.filter_search_input = None;
                            }
                        }
                    }
                    _ => {}
                }
                return Ok(false);
            }
        }

        match input {
            Event::Mouse(mouse) if mouse.kind == MouseEventKind::ScrollUp => {
                // Scroll up
                let mut scroll_speed = SCROLL_SPEED;
                if mouse.modifiers.contains(SCROLL_MODIFIER_KEY) {
                    scroll_speed += SCROLL_MODIFIER;
                }
                self.scroll_position = self.scroll_position.saturating_sub(scroll_speed);
                self.reset_drag();
            }
            Event::Mouse(mouse) if mouse.kind == MouseEventKind::ScrollDown => {
                // Scroll down
                let mut scroll_speed = SCROLL_SPEED;
                if mouse.modifiers.contains(SCROLL_MODIFIER_KEY) {
                    scroll_speed += SCROLL_MODIFIER;
                }
                self.scroll_position += scroll_speed;
                self.reset_drag()
            }
            Event::Key(key) if key.code == KeyCode::Up && key.kind == KeyEventKind::Press => {
                // Select filter list item if filter is showing, otherwise scroll console.
                if self.show_filter_menu {
                    self.selected_filter_index = self.selected_filter_index.saturating_sub(1);
                } else {
                    // Scroll up
                    let mut scroll_speed = SCROLL_SPEED;
                    if key.modifiers.contains(SCROLL_MODIFIER_KEY) {
                        scroll_speed += SCROLL_MODIFIER;
                    }
                    self.scroll_position = self.scroll_position.saturating_sub(scroll_speed);
                    self.reset_drag();
                }
            }
            Event::Key(key) if key.code == KeyCode::Down && key.kind == KeyEventKind::Press => {
                // Select filter list item if filter is showing, otherwise scroll console.
                if self.show_filter_menu {
                    let list_len = self.filters.len();
                    if self.selected_filter_index < list_len - 1 {
                        self.selected_filter_index += 1;
                    }
                } else {
                    // Scroll down
                    let mut scroll_speed = SCROLL_SPEED;
                    if key.modifiers.contains(SCROLL_MODIFIER_KEY) {
                        scroll_speed += SCROLL_MODIFIER;
                    }
                    self.scroll_position += scroll_speed;
                    self.reset_drag();
                }
            }
            Event::Key(key) if key.code == KeyCode::Left && key.kind == KeyEventKind::Press => {
                // Remove selected filter if filter menu is shown.
                if self.show_filter_menu {
                    let index = self.selected_filter_index;
                    self.filters.remove(index);
                }
            }
            Event::Key(key) if key.code == KeyCode::Right && key.kind == KeyEventKind::Press => {
                // Toggle filter if filter menu is shown.
                if self.show_filter_menu {
                    let index = self.selected_filter_index;
                    self.filters.reverse();
                    if let Some(item) = self.filters.get_mut(index) {
                        item.1 = !item.1;
                    }
                    self.filters.reverse();
                }
            }
            Event::Key(key) if key.code == KeyCode::Enter && key.kind == KeyEventKind::Press => {
                // We only need to listen to the enter key when not in search mode
                // as there is other logic that handles adding filters and disabling the mdoe.
                if self.show_filter_menu {
                    self.filter_search_mode = !self.filter_search_mode;
                }
            }
            Event::Mouse(mouse) if mouse.kind == MouseEventKind::Down(MouseButton::Left) => {
                self.reset_drag();
            }
            Event::Mouse(mouse) if mouse.kind == MouseEventKind::Drag(MouseButton::Left) => {
                // Start mouse drag
                let x = mouse.column;
                let y = mouse.row;

                if self.drag_start.is_some() {
                    self.drag_end = Some((x, y));
                } else {
                    self.drag_start = Some((x, y));
                    self.drag_end = self.drag_start;
                }
            }
            Event::Key(key)
                if key.code == KeyCode::Char('r') && key.kind == KeyEventKind::Press =>
            {
                // Reload the app
                return Ok(true);
            }
            Event::Key(key)
                if key.code == KeyCode::Char('o') && key.kind == KeyEventKind::Press =>
            {
                // Open the running app.
                open::that(format!("http://{}:{}", self.addr.addr, self.addr.port))?;
            }
            Event::Key(key)
                if key.code == KeyCode::Char('C') && key.kind == KeyEventKind::Press =>
            {
                // Copy any selected lines to the clipboard.
                let is_shift = key.modifiers.contains(KeyModifiers::SHIFT);

                if is_shift && !self.selected_lines.is_empty() {
                    let text = selection::process_selection(&mut self.selected_lines);
                    selection::set_clipboard(text);
                }
            }
            Event::Key(key)
                if key.code == KeyCode::Char('a') && key.kind == KeyEventKind::Press =>
            {
                // Select all visible lines in the console.
                let is_ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
                if is_ctrl {
                    self.drag_start = Some((0, 0));
                    self.drag_end = Some((self.console_width - 1, self.console_height - 1));
                }
            }
            Event::Key(key)
                if key.code == KeyCode::Char('f') && key.kind == KeyEventKind::Press =>
            {
                // Show filter menu and enable filter selection mode.
                if self.show_filter_menu {
                    // Reset inputs when filter menu is closed.
                    self.filter_search_mode = false;
                    self.filter_search_input = None;
                }
                self.show_filter_menu = !self.show_filter_menu;
            }
            Event::Key(key)
                if key.code == KeyCode::Char('/') && key.kind == KeyEventKind::Press =>
            {
                // Toggle more modal
                self.more_modal_open = !self.more_modal_open;
            }
            Event::Resize(_width, _height) => {
                // nothing, it should take care of itself
            }
            _ => {}
        }

        if self.scroll_position
            > self
                .num_lines_wrapping
                .saturating_sub(self.console_height + 1)
        {
            self.scroll_position = self
                .num_lines_wrapping
                .saturating_sub(self.console_height + 1);
        }

        Ok(false)
    }

    fn reset_drag(&mut self) {
        self.drag_start = None;
        self.drag_end = None;
        self.selected_lines.clear();
    }

    pub fn new_ws_message(
        &mut self,
        platform: TargetPlatform,
        message: axum::extract::ws::Message,
    ) {
        // Deccode the message and push it to our logs.
        if let axum::extract::ws::Message::Text(text) = message {
            let msg = serde_json::from_str::<ClientMsg>(text.as_str());
            match msg {
                Ok(ClientMsg::Log { level, messages }) => {
                    let level = match level.as_str() {
                        "trace" => Level::TRACE,
                        "debug" => Level::DEBUG,
                        "info" => Level::INFO,
                        "warn" => Level::WARN,
                        "error" => Level::ERROR,
                        _ => Level::INFO,
                    };

                    let content = messages.first().unwrap_or(&String::new()).clone();

                    // We don't care about logging the app's message so we directly push it instead of using tracing.
                    self.push_log(Message::new(MessageSource::App(platform), level, content));
                }
                Err(err) => {
                    tracing::error!(dx_src = ?MessageSource::Dev, "Error parsing message from {}: {}", platform, err);
                }
            }
        }
    }

    // todo: re-enable
    #[allow(unused)]
    fn is_snapped(&self) -> bool {
        true
        // let prev_scrol = self
        //     .num_lines_with_wrapping
        //     .saturating_sub(self.term_height);
        // prev_scrol == self.scroll
    }

    pub fn scroll_to_bottom(&mut self) {
        self.scroll_position = self.num_lines_wrapping.saturating_sub(self.console_height);
    }

    pub fn push_log(&mut self, message: Message) {
        self.messages.push(message);

        let snapped = self.is_snapped();
        if snapped {
            self.scroll_to_bottom();
        }
    }

    pub fn new_build_progress(&mut self, platform: TargetPlatform, update: BuildProgressUpdate) {
        self.build_progress
            .current_builds
            .entry(platform)
            .or_default()
            .update(update);

        let snapped = self.is_snapped();
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
                    if platform == &result.target_platform {
                        let stdout = child.stdout.take().unwrap();
                        let stderr = child.stderr.take().unwrap();
                        Some((stdout, stderr))
                    } else {
                        None
                    }
                });

            let platform = result.target_platform;

            let stdout = out.map(|(stdout, stderr)| RunningAppOutput {
                stdout: BufReader::new(stdout).lines(),
                stderr: BufReader::new(stderr).lines(),
                stdout_line: String::new(),
                stderr_line: String::new(),
            });

            let app = RunningApp {
                result,
                output: stdout,
            };

            self.running_apps.insert(platform, app);

            // Finish the build progress for the platform that just finished building
            if let Some(build) = self.build_progress.current_builds.get_mut(&platform) {
                build.stage = Stage::Finished;
            }
        }
    }

    pub fn render(
        &mut self,
        _opts: &Serve,
        _config: &DioxusCrate,
        _build_engine: &Builder,
        _server: &Server,
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
                let mut layout = render::TuiLayout::new(frame.size(), self.show_filter_menu);
                let (console_width, console_height) = layout.get_console_size();
                self.console_width = console_width;
                self.console_height = console_height;

                // Render the decor first as some of it (such as backgrounds) may be rendered on top of.
                layout.render_decor(frame, self.show_filter_menu);

                // Get only the enabled filters.
                let mut enabled_filters = self.filters.clone();
                enabled_filters.retain(|f| f.1);
                let enabled_filters = enabled_filters
                    .iter()
                    .map(|f| f.0.clone())
                    .collect::<Vec<String>>();

                // Render console, we need the number of wrapping lines for scroll.
                self.num_lines_wrapping = layout.render_console(
                    frame,
                    self.scroll_position,
                    &self.messages,
                    &enabled_filters,
                );

                layout.render_selection(
                    frame,
                    self.drag_start,
                    self.drag_end,
                    &mut self.selected_lines,
                );

                if self.show_filter_menu {
                    layout.render_filter_menu(
                        frame,
                        &self.filters,
                        self.selected_filter_index,
                        self.filter_search_mode,
                        self.filter_search_input.as_ref(),
                    );
                }

                layout.render_status_bar(
                    frame,
                    self.is_cli_release,
                    self.platform,
                    &self.build_progress,
                    self.more_modal_open,
                    self.show_filter_menu,
                );

                if self.more_modal_open {
                    layout.render_more_modal(frame);
                }
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
}

#[derive(Default, Debug, PartialEq)]
pub struct ActiveBuild {
    stage: Stage,
    progress: f64,
    failed: Option<String>,
}

impl ActiveBuild {
    fn update(&mut self, update: BuildProgressUpdate) {
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
            Stage::Finished => "finished! √ ",
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
        let mut stdout = stdout();
        _ = stdout.execute(LeaveAlternateScreen);
        _ = stdout.execute(DisableMouseCapture);
        _ = stdout.execute(Show);
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
    output: Option<RunningAppOutput>,
}

struct RunningAppOutput {
    stdout: Lines<BufReader<ChildStdout>>,
    stderr: Lines<BufReader<ChildStderr>>,
    stdout_line: String,
    stderr_line: String,
}
