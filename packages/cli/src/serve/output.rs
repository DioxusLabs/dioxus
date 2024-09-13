use crate::{
    builder::{BuildMessage, BuildUpdateProgress, Platform, Stage, UpdateStage},
    cli::serve::ServeArgs,
    dioxus_crate::DioxusCrate,
    serve::{Builder, Watcher},
    tracer::CLILogControl,
    TraceMsg, TraceSrc,
};
use crossterm::{
    cursor::{Hide, Show},
    event::{Event, EventStream, KeyCode, KeyEventKind, KeyModifiers},
    terminal::{
        disable_raw_mode, enable_raw_mode, Clear, ClearType, EnterAlternateScreen,
        LeaveAlternateScreen,
    },
    tty::IsTty,
    ExecutableCommand,
};
use dioxus_devtools_types::ClientMsg;
// use dioxus_cli_config::{AddressArguments, Platform};
// use dioxus_hot_reload::ClientMsg;
use futures_util::{
    future::{select_all, OptionFuture},
    Future, FutureExt, StreamExt,
};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph},
    TerminalOptions, Viewport,
};
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    fmt::Display,
    io::{self, stdout},
    rc::Rc,
    time::{Duration, Instant},
};

use tracing::Level;

use super::{AppHandle, DevServer, ServeUpdate};

mod render;

// How many lines should be scroll on each mouse scroll or arrow key input.
const SCROLL_SPEED: u16 = 2;
// Speed added to `SCROLL_SPEED` when the modifier key is held during scroll.
const SCROLL_MODIFIER: u16 = 4;
// Scroll modifier key.
const SCROLL_MODIFIER_KEY: KeyModifiers = KeyModifiers::SHIFT;

pub struct Output {
    term: Rc<RefCell<Option<TerminalBackend>>>,

    // optional since when there's no tty there's no eventstream to read from - just stdin
    events: Option<EventStream>,

    pub(crate) build_progress: BuildProgress,
    // running_apps: HashMap<Platform, RunningApp>,

    // A list of all messages from build, dev, app, and more.
    messages: Vec<TraceMsg>,

    num_lines_wrapping: u16,
    scroll_position: u16,
    console_width: u16,
    console_height: u16,

    more_modal_open: bool,
    anim_start: Instant,

    interactive: bool,
    _is_cli_release: bool,
    platform: Platform,
    // addr: AddressArguments,

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
    pub(crate) fn start(cfg: &ServeArgs) -> io::Result<Self> {
        let interactive = cfg.interactive_tty();
        let mut events = None;

        if interactive {
            // log_control.output_enabled.store(true, Ordering::SeqCst);
            enable_raw_mode()?;
            stdout().execute(EnterAlternateScreen)?.execute(Hide)?;

            // workaround for ci where the terminal is not fully initialized
            // this stupid bug
            // https://github.com/crossterm-rs/crossterm/issues/659
            events = Some(EventStream::new());
        };

        // set the panic hook to fix the terminal
        set_fix_term_hook();

        // Fix the vscode scrollback issue
        fix_xtermjs_scrollback();

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

        let is_cli_release = crate::build_info::PROFILE == "release";

        if !is_cli_release {
            if let Some(hash) = crate::build_info::GIT_COMMIT_HASH_SHORT {
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
            _is_cli_release: is_cli_release,
            platform,
            messages: Vec::new(),
            more_modal_open: false,
            build_progress: Default::default(),
            // running_apps: HashMap::new(),
            scroll_position: 0,
            num_lines_wrapping: 0,
            console_width: 0,
            console_height: 0,
            anim_start: Instant::now(),
            // addr: cfg.address.clone(),

            // Filter
            show_filter_menu: false,
            filters: Vec::new(),
            selected_filter_index: 0,
            filter_search_input: None,
            filter_search_mode: false,
        })
    }

    /// Add a message from stderr to the logs
    pub fn push_stderr(&mut self, platform: Platform, stderr: String) {
        // self.running_apps
        //     .get_mut(&platform)
        //     .unwrap()
        //     .output
        //     .as_mut()
        //     .unwrap()
        //     .stderr_line
        //     .push_str(&stderr);

        self.messages.push(TraceMsg {
            source: TraceSrc::App(platform),
            level: Level::ERROR,
            content: stderr,
        });

        if self.is_snapped() {
            self.scroll_to_bottom();
        }
    }

    /// Add a message from stdout to the logs
    pub fn push_stdout(&mut self, platform: Platform, stdout: String) {
        // self.running_apps
        //     .get_mut(&platform)
        //     .unwrap()
        //     .output
        //     .as_mut()
        //     .unwrap()
        //     .stdout_line
        //     .push_str(&stdout);

        self.messages.push(TraceMsg {
            source: TraceSrc::App(platform),
            level: Level::INFO,
            content: stdout,
        });

        if self.is_snapped() {
            self.scroll_to_bottom();
        }
    }

    /// Wait for either the ctrl_c handler or the next event
    ///
    /// Why is the ctrl_c handler here?
    ///
    /// Also tick animations every few ms
    pub(crate) async fn wait(&mut self) -> ServeUpdate {
        let event = tokio::select! {
            Some(Some(Ok(event))) = OptionFuture::from(self.events.as_mut().map(|f| f.next())) => event,
            else => futures_util::future::pending().await
        };

        ServeUpdate::TuiInput { event }
    }

    pub(crate) fn shutdown(&mut self) -> io::Result<()> {
        // if we're a tty then we need to disable the raw mode
        if self.interactive {
            // self.log_control
            //     .output_enabled
            //     .store(false, Ordering::SeqCst);
            disable_raw_mode()?;
            stdout().execute(LeaveAlternateScreen)?.execute(Show)?;
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
            if msg.source != TraceSrc::Cargo {
                println!("[{}] {}: {}", msg.source, msg.level, msg.content);
            } else {
                println!("{}", msg.content);
            }
        }
    }

    /// Handle an input event, returning `true` if the event should cause the program to restart.
    pub(crate) fn handle_input(&mut self, input: Event) -> io::Result<bool> {
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
                }
            }
            Event::Key(key) if key.code == KeyCode::Down && key.kind == KeyEventKind::Press => {
                // Select filter list item if filter is showing, otherwise scroll console.
                if self.show_filter_menu {
                    let list_len = self.filters.len();
                    if self.selected_filter_index + 1 < list_len {
                        self.selected_filter_index += 1;
                    }
                } else {
                    // Scroll down
                    let mut scroll_speed = SCROLL_SPEED;
                    if key.modifiers.contains(SCROLL_MODIFIER_KEY) {
                        scroll_speed += SCROLL_MODIFIER;
                    }
                    self.scroll_position += scroll_speed;
                }
            }
            Event::Key(key) if key.code == KeyCode::Left && key.kind == KeyEventKind::Press => {
                // Remove selected filter if filter menu is shown.
                if self.show_filter_menu {
                    let index = self.selected_filter_index;
                    if self.filters.get(index).is_some() {
                        self.filters.remove(index);
                    }
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
                // as there is other logic that handles adding filters and disabling the mode.
                if self.show_filter_menu {
                    self.filter_search_mode = !self.filter_search_mode;
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
                // open::that(format!("http://{}:{}", self.addr, self.port))?;
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

        if self.scroll_position > self.num_lines_wrapping.saturating_sub(self.console_height) {
            self.scroll_to_bottom();
        }

        Ok(false)
    }

    pub(crate) fn new_ws_message(
        &mut self,
        platform: Platform,
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
                    self.push_log(TraceMsg::new(TraceSrc::App(platform), level, content));
                }
                Err(err) => {
                    tracing::error!(dx_src = ?TraceSrc::Dev, "Error parsing message from {}: {}", platform, err);
                }
            }
        }
    }

    // pub(crate) fn scroll_to_bottom(&mut self) {
    //     self.scroll = (self.num_lines_with_wrapping).saturating_sub(self.term_height);
    // }

    pub(crate) fn push_inner_log(&mut self, msg: String) {
        // self.push_log(
        //     LogSource::Internal,
        //     crate::builder::BuildMessage {
        //         level: tracing::Level::INFO,
        //         message: crate::builder::MessageType::Text(msg),
        //         source: crate::builder::MessageSource::Dev,
        //     },
        // );
    }

    pub(crate) fn new_build_logs(&mut self, platform: Platform, update: BuildUpdateProgress) {
        //         let snapped = self.is_snapped(LogSource::Target(platform));

        //         // when the build is finished, switch to the console
        //         if update.stage == Stage::Finished {
        //             self.tab = Tab::Console;
        //         }
    }

    fn is_snapped(&self) -> bool {
        true
    }

    pub fn scroll_to_bottom(&mut self) {
        self.scroll_position = self.num_lines_wrapping.saturating_sub(self.console_height);
    }

    pub fn push_log(&mut self, message: TraceMsg) {
        self.messages.push(message);

        if self.is_snapped() {
            self.scroll_to_bottom();
        }
    }

    // pub fn new_build_progress(&mut self, platform: Platform, update: BuildProgress) {
    //     self.build_progress
    //         .current_builds
    //         .entry(platform)
    //         .or_default()
    //         .update(update);

    //     if self.is_snapped() {
    //         self.scroll_to_bottom();
    //     }
    // }

    pub(crate) fn new_ready_app(&mut self, handle: &AppHandle) {
        // Finish the build progress for the platform that just finished building
        if let Some(build) = self
            .build_progress
            .current_builds
            .get_mut(&handle.app.build.platform())
        {
            build.stage = Stage::Finished;
        }
    }

    pub fn render(
        &mut self,
        _opts: &ServeArgs,
        _config: &DioxusCrate,
        _build_engine: &Builder,
        _server: &DevServer,
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
                    self.platform,
                    &self.build_progress,
                    self.more_modal_open,
                    self.show_filter_menu,
                    &self._dx_version,
                );

                if self.more_modal_open {
                    layout.render_more_modal(frame);
                }

                layout.render_current_scroll(
                    self.scroll_position,
                    self.num_lines_wrapping,
                    self.console_height,
                    frame,
                );
            });
    }

    // pub(crate) fn render(
    //     &mut self,
    //     _args: &ServeArgs,
    //     _krate: &DioxusCrate,
    //     _builder: &Builder,
    //     server: &DevServer,
    //     _watcher: &Watcher,
    // ) {
    //     // just drain the build logs
    //     if !self.interactive {
    //         self.drain_print_logs();
    //         return;
    //     }

    //     // Keep the animation track in terms of 100ms frames - the frame should be a number between 0 and 10
    //     // todo: we want to use this somehow to animate things...
    //     let elapsed = self.anim_start.elapsed().as_millis() as f32;
    //     let num_frames = elapsed / 100.0;
    //     let _frame_step = (num_frames % 10.0) as usize;

    //     _ = self
    //         .term
    //         .clone()
    //         .borrow_mut()
    //         .as_mut()
    //         .unwrap()
    //         .draw(|frame| {
    //             let mut layout = render::TuiLayout::new(frame.size(), self.show_filter_menu);
    //             let (console_width, console_height) = layout.get_console_size();
    //             self.console_width = console_width;
    //             self.console_height = console_height;

    //             // Render the decor first as some of it (such as backgrounds) may be rendered on top of.
    //             layout.render_decor(frame, self.show_filter_menu);

    //             // Get only the enabled filters.
    //             let mut enabled_filters = self.filters.clone();
    //             enabled_filters.retain(|f| f.1);
    //             let enabled_filters = enabled_filters
    //                 .iter()
    //                 .map(|f| f.0.clone())
    //                 .collect::<Vec<String>>();

    //             // Render console, we need the number of wrapping lines for scroll.
    //             self.num_lines_wrapping = layout.render_console(
    //                 frame,
    //                 self.scroll_position,
    //                 &self.messages,
    //                 &enabled_filters,
    //             );

    //             // // Render a border for the header
    //             // frame.render_widget(Block::default().borders(Borders::BOTTOM), body[0]);

    //             // Render the metadata
    //             let mut spans: Vec<Span> = vec![
    //                 Span::from(if self.is_cli_release { "dx" } else { "dx-dev" }).green(),
    //                 Span::from(" ").green(),
    //                 Span::from("serve").green(),
    //                 Span::from(" | ").white(),
    //                 Span::from(self.platform.to_string()).green(),
    //                 Span::from(" | ").white(),
    //             ];

    //             // If there is build progress, display that next to the platform
    //             if !self.build_progress.current_builds.is_empty() {
    //                 if self
    //                     .build_progress
    //                     .current_builds
    //                     .values()
    //                     .any(|b| b.failed.is_some())
    //                 {
    //                     spans.push(Span::from("build failed ❌").red());
    //                 } else {
    //                     spans.push(Span::from("status: ").green());
    //                     let build = self
    //                         .build_progress
    //                         .current_builds
    //                         .values()
    //                         .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
    //                         .unwrap();

    //                     spans.extend_from_slice(&build.spans(Rect::new(
    //                         0,
    //                         0,
    //                         build.max_layout_size(),
    //                         1,
    //                     )));
    //                 }
    //             }

    //             frame.render_widget(Paragraph::new(Line::from(spans)).left_aligned(), header[0]);

    //             // Split apart the body into a center and a right side
    //             // We only want to show the sidebar if there's enough space
    //             if listening_len > 0 {
    //                 frame.render_widget(
    //                     Paragraph::new(Line::from(vec![
    //                         Span::from("listening at ").dark_gray(),
    //                         Span::from(format!("http://{}", server.ip).as_str()).gray(),
    //                     ])),
    //                     header[1],
    //                 )
    //             }

    //             if self.show_filter_menu {
    //                 layout.render_filter_menu(
    //                     frame,
    //                     &self.filters,
    //                     self.selected_filter_index,
    //                     self.filter_search_mode,
    //                     self.filter_search_input.as_ref(),
    //                 );
    //             }

    //             layout.render_status_bar(
    //                 frame,
    //                 self.platform,
    //                 &self.build_progress,
    //                 self.more_modal_open,
    //                 self.show_filter_menu,
    //                 &self._dx_version,
    //             );

    //             if self.more_modal_open {
    //                 layout.render_more_modal(frame);
    //             }

    //             layout.render_current_scroll(
    //                 self.scroll_position,
    //                 self.num_lines_wrapping,
    //                 self.console_height,
    //                 frame,
    //             );
    //         });
    // }

    // fn render_fly_modal(&mut self, frame: &mut Frame, area: Rect) {
    //     if !self.fly_modal_open {
    //         return;
    //     }

    //     // Create a frame slightly smaller than the area
    //     let panel = Layout::default()
    //         .direction(Direction::Vertical)
    //         .constraints([Constraint::Fill(1)].as_ref())
    //         .split(area)[0];

    //     // Wipe the panel
    //     frame.render_widget(Clear, panel);
    //     frame.render_widget(Block::default().borders(Borders::ALL), panel);

    //     let modal = Paragraph::new("Under construction, please check back at a later date!\n")
    //         .alignment(Alignment::Center);
    //     frame.render_widget(modal, panel);
    // }

    // fn push_log(&mut self, platform: impl Into<LogSource>, message: BuildMessage) {
    //     let source = platform.into();
    //     let snapped = self.is_snapped();

    //     match source {
    //         LogSource::Internal => self.build_progress.internal_logs.push(message),
    //         LogSource::Target(platform) => self
    //             .build_progress
    //             .current_builds
    //             .entry(platform)
    //             .or_default()
    //             .stdout_logs
    //             .push(message),
    //     }

    //     if snapped {
    //         self.scroll_to_bottom();
    //     }
    // }

    // // todo: re-enable
    // #[allow(unused)]
    // fn is_snapped(&self) -> bool {
    //     true
    // }

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
pub(crate) struct ActiveBuild {
    stage: Stage,
    progress: f64,
    failed: Option<String>,
}

impl ActiveBuild {
    // fn update(&mut self, update: BuildUpdateProgress) {
    //     match update.update {
    //         UpdateStage::Start => {
    //             // If we are already past the stage, don't roll back, but allow a fresh build to update.
    //             if self.stage > update.stage && self.stage < Stage::Finished {
    //                 return;
    //             }
    //             self.stage = update.stage;
    //             self.progress = 0.0;
    //             self.failed = None;
    //         }
    //         UpdateStage::SetProgress(progress) => {
    //             self.progress = progress;
    //         }
    //         UpdateStage::Failed(failed) => {
    //             self.stage = Stage::Finished;
    //             self.failed = Some(failed.clone());
    //         }
    //     }
    // }

    fn make_spans(&self, area: Rect) -> Vec<Span> {
        let mut spans = Vec::new();

        let message = match self.stage {
            Stage::Initializing => "Initializing...",
            Stage::InstallingWasmTooling => "Configuring...",
            Stage::Compiling => "Compiling...",
            Stage::OptimizingWasm => "Optimizing...",
            Stage::OptimizingAssets => "Copying Assets...",
            Stage::Finished => "Build finished! 🎉 ",
        };

        let progress = format!(" {}%", (self.progress * 100.0) as u8);

        if area.width >= self.max_layout_size() {
            match self.stage {
                Stage::Finished => spans.push(Span::from(message).light_yellow()),
                _ => spans.push(Span::from(message).light_yellow()),
            }

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
        _ = stdout.execute(Show);
        original_hook(info);
    }));
}

/// clearing and writing a new line fixes the xtermjs scrollback issue
fn fix_xtermjs_scrollback() {
    _ = crossterm::execute!(std::io::stdout(), Clear(ClearType::All));
    println!();
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum LogSource {
    Internal,
    Target(Platform),
}

impl Display for LogSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogSource::Internal => write!(f, "CLI"),
            LogSource::Target(platform) => write!(f, "{platform}"),
        }
    }
}

impl From<Platform> for LogSource {
    fn from(platform: Platform) -> Self {
        LogSource::Target(platform)
    }
}

#[derive(Default)]
pub(crate) struct BuildProgress {
    internal_logs: Vec<BuildMessage>,
    current_builds: HashMap<Platform, ActiveBuild>,
}

// impl BuildProgress {
//     pub(crate) fn progress(&self) -> f64 {
//         self.build_logs
//             .values()
//             .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
//             .map(|build| match build.stage {
//                 Stage::Initializing => 0.0,
//                 Stage::InstallingWasmTooling => 0.0,
//                 Stage::Compiling => build.progress,
//                 Stage::OptimizingWasm | Stage::OptimizingAssets | Stage::Finished => 1.0,
//             })
//             .unwrap_or_default()
//     }
// }

// #[derive(Default)]
// pub struct BuildProgress {
//     current_builds: HashMap<Platform, ActiveBuild>,
// }

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
