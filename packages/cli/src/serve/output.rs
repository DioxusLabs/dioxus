use crate::{
    serve::{ansi_buffer::AnsiStringBuffer, Builder, ServeUpdate, Watcher, WebServer},
    BuildStage, BuildUpdate, DioxusCrate, Platform, ServeArgs, TraceContent, TraceMsg, TraceSrc,
};
use crossterm::{
    cursor::{Hide, Show},
    event::{
        DisableBracketedPaste, DisableFocusChange, EnableBracketedPaste, EnableFocusChange, Event,
        EventStream, KeyCode, KeyEvent, KeyEventKind, KeyModifiers,
    },
    terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType},
    ExecutableCommand,
};
use ratatui::{
    prelude::*,
    widgets::{Block, BorderType, Borders, LineGauge, Paragraph, Wrap},
    TerminalOptions, Viewport,
};
use std::{
    cell::RefCell,
    collections::VecDeque,
    io::{self, stdout},
    rc::Rc,
    time::Duration,
};
use tracing::Level;

const TICK_RATE_MS: u64 = 100;
const VIEWPORT_MAX_WIDTH: u16 = 100;
const VIEWPORT_HEIGHT_SMALL: u16 = 5;
const VIEWPORT_HEIGHT_BIG: u16 = 12;

/// The TUI that drives the console output.
///
/// We try not to store too much state about the world here, just the state about the tui itself.
/// This is to prevent out-of-sync issues with the rest of the build engine and to use the components
/// of the serve engine as the source of truth.
///
/// Please please, do not add state here that does not belong here. We should only be storing state
/// here that is used to change how we display *other* state. Things like throbbers, modals, etc.
pub struct Output {
    term: Rc<RefCell<Option<Terminal<CrosstermBackend<io::Stdout>>>>>,
    events: Option<EventStream>,

    // A list of all messages from build, dev, app, and more.
    more_modal_open: bool,
    interactive: bool,
    platform: Platform,

    // Whether to show verbose logs or not
    // We automatically hide "debug" logs if verbose is false (only showing "info" / "warn" / "error")
    verbose: bool,
    trace: bool,

    // Pending logs
    pending_logs: VecDeque<TraceMsg>,

    dx_version: String,
    tick_animation: bool,

    tick_interval: tokio::time::Interval,

    // ! needs to be wrapped in an &mut since `render stateful widget` requires &mut... but our
    // "render" method only borrows &self (for no particular reason at all...)
    throbber: RefCell<throbber_widgets_tui::ThrobberState>,
}

#[allow(unused)]
#[derive(Clone, Copy)]
struct RenderState<'a> {
    opts: &'a ServeArgs,
    krate: &'a DioxusCrate,
    build_engine: &'a Builder,
    server: &'a WebServer,
    watcher: &'a Watcher,
}

impl Output {
    pub(crate) fn start(cfg: &ServeArgs) -> io::Result<Self> {
        let mut output = Self {
            term: Rc::new(RefCell::new(
                Terminal::with_options(
                    CrosstermBackend::new(stdout()),
                    TerminalOptions {
                        viewport: Viewport::Inline(VIEWPORT_HEIGHT_SMALL),
                    },
                )
                .ok(),
            )),
            interactive: cfg.is_interactive_tty(),
            dx_version: format!(
                "{}-{}",
                env!("CARGO_PKG_VERSION"),
                crate::dx_build_info::GIT_COMMIT_HASH_SHORT.unwrap_or("main")
            ),
            platform: cfg.build_arguments.platform.expect("To be resolved by now"),
            events: None,
            // messages: Vec::new(),
            more_modal_open: false,
            pending_logs: VecDeque::new(),
            throbber: RefCell::new(throbber_widgets_tui::ThrobberState::default()),
            trace: cfg.trace,
            verbose: cfg.verbose,
            tick_animation: false,
            tick_interval: {
                let mut interval = tokio::time::interval(Duration::from_millis(TICK_RATE_MS));
                interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
                interval
            },
        };

        output.startup()?;

        Ok(output)
    }

    /// Call the startup functions that might mess with the terminal settings.
    /// This is meant to be paired with "shutdown" to restore the terminal to its original state.
    fn startup(&mut self) -> io::Result<()> {
        if self.interactive {
            // set the panic hook to fix the terminal in the event of a panic
            // The terminal might be left in a wonky state if a panic occurs, and we don't want it to be completely broken
            let original_hook = std::panic::take_hook();
            std::panic::set_hook(Box::new(move |info| {
                _ = disable_raw_mode();
                _ = stdout().execute(Show);
                original_hook(info);
            }));

            enable_raw_mode()?;
            stdout()
                .execute(Hide)?
                .execute(EnableFocusChange)?
                .execute(EnableBracketedPaste)?;

            // Initialize the event stream here - this is optional because an EvenStream in a non-interactive
            // terminal will cause a panic instead of simply doing nothing.
            // https://github.com/crossterm-rs/crossterm/issues/659
            self.events = Some(EventStream::new());
        }

        Ok(())
    }

    /// Call the shutdown functions that might mess with the terminal settings - see the related code
    /// in "startup" for more details about what we need to unset
    pub(crate) fn shutdown(&self) -> io::Result<()> {
        if self.interactive {
            stdout()
                .execute(Show)?
                .execute(DisableFocusChange)?
                .execute(DisableBracketedPaste)?;
            disable_raw_mode()?;

            // print a few lines to not cut off the output
            for _ in 0..3 {
                println!();
            }
        }

        Ok(())
    }

    pub(crate) async fn wait(&mut self) -> ServeUpdate {
        use futures_util::future::OptionFuture;
        use futures_util::StreamExt;

        // Wait for the next user event or animation tick
        loop {
            let next = OptionFuture::from(self.events.as_mut().map(|f| f.next()));
            let event = tokio::select! {
                biased; // Always choose the event over the animation tick to not lose the event
                Some(Some(Ok(event))) = next => event,
                _ = self.tick_interval.tick(), if self.tick_animation => {
                    self.throbber.borrow_mut().calc_next();
                    return ServeUpdate::Redraw
                },
                else => futures_util::future::pending().await
            };

            match self.handle_input(event) {
                Ok(Some(update)) => return update,
                Err(ee) => {
                    return ServeUpdate::Exit {
                        error: Some(Box::new(ee)),
                    }
                }
                Ok(None) => {}
            }
        }
    }

    /// Handle an input event, returning `true` if the event should cause the program to restart.
    fn handle_input(&mut self, input: Event) -> io::Result<Option<ServeUpdate>> {
        // handle ctrlc
        if let Event::Key(key) = input {
            if let KeyCode::Char('c') = key.code {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    return Ok(Some(ServeUpdate::Exit { error: None }));
                }
            }
        }

        match input {
            Event::Key(key) if key.kind == KeyEventKind::Press => self.handle_keypress(key),
            _ => Ok(Some(ServeUpdate::Redraw)),
        }
    }

    fn handle_keypress(&mut self, key: KeyEvent) -> io::Result<Option<ServeUpdate>> {
        match key.code {
            KeyCode::Char('r') => return Ok(Some(ServeUpdate::RequestRebuild)),
            KeyCode::Char('o') => return Ok(Some(ServeUpdate::OpenApp)),
            KeyCode::Char('p') => return Ok(Some(ServeUpdate::ToggleShouldRebuild)),
            KeyCode::Char('v') => {
                self.verbose = !self.verbose;
                tracing::info!(
                    "Verbose logging is now {}",
                    if self.verbose { "on" } else { "off" }
                );
            }
            KeyCode::Char('t') => {
                self.trace = !self.trace;
                tracing::info!("Tracing is now {}", if self.trace { "on" } else { "off" });
            }

            KeyCode::Char('c') => {
                stdout()
                    .execute(Clear(ClearType::All))?
                    .execute(Clear(ClearType::Purge))?;
                _ = self.term.borrow_mut().as_mut().map(|t| t.clear());
            }

            // Toggle the more modal by swapping the the terminal with a new one
            // This is a bit of a hack since crossterm doesn't technically support changing the
            // size of an inline viewport.
            KeyCode::Char('/') => {
                if let Some(terminal) = self.term.borrow_mut().as_mut() {
                    // Toggle the more modal, which will change our current viewport height
                    self.more_modal_open = !self.more_modal_open;

                    // Clear the terminal before resizing it, such that it doesn't tear
                    terminal.clear()?;

                    // And then set the new viewport, which essentially mimics a resize
                    *terminal = Terminal::with_options(
                        CrosstermBackend::new(stdout()),
                        TerminalOptions {
                            viewport: Viewport::Inline(self.viewport_current_height()),
                        },
                    )?;
                }
            }

            _ => {}
        }

        // Out of safety, we always redraw, since it's relatively cheap operation
        Ok(Some(ServeUpdate::Redraw))
    }

    /// Push a TraceMsg to be printed on the next render
    pub fn push_log(&mut self, message: TraceMsg) {
        self.pending_logs.push_front(message);
    }

    pub fn push_cargo_log(&mut self, message: cargo_metadata::CompilerMessage) {
        use cargo_metadata::diagnostic::DiagnosticLevel;

        if self.trace
            || matches!(
                message.message.level,
                DiagnosticLevel::Error | DiagnosticLevel::FailureNote
            )
        {
            self.push_log(TraceMsg::cargo(message));
        }
    }

    /// Add a message from stderr to the logs
    /// This will queue the stderr message as a TraceMsg and print it on the next render
    /// We'll use the `App` TraceSrc for the msg, and whatever level is provided
    pub fn push_stdio(&mut self, platform: Platform, msg: String, level: Level) {
        self.push_log(TraceMsg::text(TraceSrc::App(platform), level, msg));
    }

    /// Push a message from the websocket to the logs
    pub fn push_ws_message(&mut self, platform: Platform, message: axum::extract::ws::Message) {
        use dioxus_devtools_types::ClientMsg;

        // We can only handle text messages from the websocket...
        let axum::extract::ws::Message::Text(text) = message else {
            return;
        };

        // ...and then decode them into a ClientMsg
        let res = serde_json::from_str::<ClientMsg>(text.as_str());

        // Client logs being errors aren't fatal, but we should still report them them
        let ClientMsg::Log { level, messages } = match res {
            Ok(msg) => msg,
            Err(err) => {
                tracing::error!(dx_src = ?TraceSrc::Dev, "Error parsing message from {}: {}", platform, err);
                return;
            }
        };

        // FIXME(jon): why are we pulling only the first message here?
        let content = messages.first().unwrap_or(&String::new()).clone();

        let level = match level.as_str() {
            "trace" => Level::TRACE,
            "debug" => Level::DEBUG,
            "info" => Level::INFO,
            "warn" => Level::WARN,
            "error" => Level::ERROR,
            _ => Level::INFO,
        };

        // We don't care about logging the app's message so we directly push it instead of using tracing.
        self.push_log(TraceMsg::text(TraceSrc::App(platform), level, content));
    }

    /// Change internal state based on the build engine's update
    ///
    /// We want to keep internal state as limited as possible, so currently we're only setting our
    /// animation tick. We could, in theory, just leave animation running and have no internal state,
    /// but that seems a bit wasteful. We might eventually change this to be more of a "requestAnimationFrame"
    /// approach, but then we'd need to do that *everywhere* instead of simply performing a react-like
    /// re-render when external state changes. Ratatui will diff the intermediate buffer, so we at least
    /// we won't be drawing it.
    pub(crate) fn new_build_update(&mut self, update: &BuildUpdate) {
        match update {
            BuildUpdate::CompilerMessage { .. } => {}
            BuildUpdate::Progress { .. } => self.tick_animation = true,
            BuildUpdate::BuildReady { .. } => self.tick_animation = false,
            BuildUpdate::BuildFailed { .. } => self.tick_animation = false,
        }
    }

    /// Render the current state of everything to the console screen
    pub fn render(
        &mut self,
        opts: &ServeArgs,
        config: &DioxusCrate,
        build_engine: &Builder,
        server: &WebServer,
        watcher: &Watcher,
    ) {
        if !self.interactive {
            return;
        }

        // Get a handle to the terminal with a different lifetime so we can continue to call &self methods
        let owned_term = self.term.clone();
        let mut term = owned_term.borrow_mut();
        let Some(term) = term.as_mut() else {
            return;
        };

        // First, dequeue any logs that have built up from event handling
        _ = self.drain_logs(term);

        // Then, draw the frame, passing along all the state of the TUI so we can render it properly
        _ = term.draw(|frame| {
            self.render_frame(
                frame,
                RenderState {
                    opts,
                    krate: config,
                    build_engine,
                    server,
                    watcher,
                },
            );
        });
    }

    fn render_frame(&self, frame: &mut Frame, state: RenderState) {
        // Use the max size of the viewport, but shrunk to a sensible max width
        let mut area = frame.area();
        area.width = area.width.clamp(0, VIEWPORT_MAX_WIDTH);

        let [_top, body, bottom] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Fill(1),
            Constraint::Length(1),
        ])
        .horizontal_margin(1)
        .areas(area);

        self.render_borders(frame, area);
        self.render_body(frame, body, state);
        self.render_bottom_row(frame, bottom, state);
        self.render_body_title(frame, _top, state);
    }

    fn render_body_title(&self, frame: &mut Frame<'_>, area: Rect, _state: RenderState) {
        frame.render_widget(
            Line::from(vec![
                " ".dark_gray(),
                match self.more_modal_open {
                    true => "/:more".light_yellow(),
                    false => "/:more".dark_gray(),
                },
                " ".dark_gray(),
            ])
            .right_aligned(),
            area,
        );
    }

    fn render_body(&self, frame: &mut Frame<'_>, area: Rect, state: RenderState) {
        let [_title, body, more, _foot] = Layout::vertical([
            Constraint::Length(0),
            Constraint::Length(VIEWPORT_HEIGHT_SMALL - 2),
            Constraint::Fill(1),
            Constraint::Length(0),
        ])
        .horizontal_margin(1)
        .areas(area);

        let [col1, col2] = Layout::horizontal([Constraint::Length(50), Constraint::Fill(1)])
            .horizontal_margin(1)
            .areas(body);

        self.render_gauges(frame, col1, state);
        self.render_stats(frame, col2, state);

        if self.more_modal_open {
            self.render_more_modal(frame, more, state);
        }
    }

    fn render_gauges(&self, frame: &mut Frame<'_>, area: Rect, state: RenderState) {
        let [gauge_area, _margin] =
            Layout::horizontal([Constraint::Fill(1), Constraint::Length(3)]).areas(area);

        let [app_progress, second_progress, status_line]: [_; 3] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .areas(gauge_area);

        self.render_single_gauge(
            frame,
            app_progress,
            state.build_engine.compile_progress(),
            "App:    ",
            state,
            state.build_engine.compile_duration(),
        );

        if state.build_engine.request.build.fullstack {
            self.render_single_gauge(
                frame,
                second_progress,
                state.build_engine.server_compile_progress(),
                "Server: ",
                state,
                state.build_engine.compile_duration(),
            );
        } else {
            self.render_single_gauge(
                frame,
                second_progress,
                state.build_engine.bundle_progress(),
                "Bundle: ",
                state,
                state.build_engine.bundle_duration(),
            );
        }

        let mut lines = vec!["Status:  ".white()];
        match &state.build_engine.stage {
            BuildStage::Initializing => lines.push("Initializing".yellow()),
            BuildStage::Starting { .. } => lines.push("Starting build".yellow()),
            BuildStage::InstallingTooling {} => lines.push("Installing tooling".yellow()),
            BuildStage::Compiling {
                current,
                total,
                krate,
                ..
            } => {
                lines.push("Compiling ".yellow());
                lines.push(format!("{current}/{total} ").gray());
                lines.push(krate.as_str().dark_gray())
            }
            BuildStage::OptimizingWasm {} => lines.push("Optimizing wasm".yellow()),
            BuildStage::RunningBindgen {} => lines.push("Running wasm-bindgen".yellow()),
            BuildStage::Bundling {} => lines.push("Bundling app".yellow()),
            BuildStage::CopyingAssets {
                current,
                total,
                path,
            } => {
                lines.push("Copying asset ".yellow());
                lines.push(format!("{current}/{total} ").gray());
                if let Some(name) = path.file_name().and_then(|f| f.to_str()) {
                    lines.push(name.dark_gray())
                }
            }
            BuildStage::Success => {
                lines.push("Serving ".yellow());
                lines.push(state.krate.executable_name().white());
                lines.push(" 🚀 ".green());
                if let Some(comp_time) = state.build_engine.total_build_time() {
                    lines.push(format!("{:.1}s", comp_time.as_secs_f32()).dark_gray());
                }
            }
            BuildStage::Failed => lines.push("Failed".red()),
            BuildStage::Aborted => lines.push("Aborted".red()),
            BuildStage::Restarting => lines.push("Restarting".yellow()),
        };

        frame.render_widget(Line::from(lines), status_line);
    }

    fn render_single_gauge(
        &self,
        frame: &mut Frame<'_>,
        area: Rect,
        value: f64,
        label: &str,
        state: RenderState,
        time_taken: Option<Duration>,
    ) {
        let failed = state.build_engine.stage == BuildStage::Failed;
        let value = if failed { 1.0 } else { value.clamp(0.0, 1.0) };

        let [gauge_row, _, icon] = Layout::horizontal([
            Constraint::Fill(1),
            Constraint::Length(2),
            Constraint::Length(10),
        ])
        .areas(area);

        frame.render_widget(
            LineGauge::default()
                .filled_style(Style::default().fg(match value {
                    1.0 if failed => Color::Red,
                    1.0 => Color::Green,
                    _ => Color::Yellow,
                }))
                .unfilled_style(Style::default().fg(Color::DarkGray))
                .label(label.gray())
                .line_set(symbols::line::THICK)
                .ratio(if !failed { value } else { 1.0 }),
            gauge_row,
        );

        let [throbber_frame, time_frame] = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(3), Constraint::Fill(1)])
            .areas(icon);

        if value != 1.0 {
            let throb = throbber_widgets_tui::Throbber::default()
                .style(ratatui::style::Style::default().fg(ratatui::style::Color::Cyan))
                .throbber_style(
                    ratatui::style::Style::default()
                        .fg(ratatui::style::Color::White)
                        .add_modifier(ratatui::style::Modifier::BOLD),
                )
                .throbber_set(throbber_widgets_tui::BLACK_CIRCLE)
                .use_type(throbber_widgets_tui::WhichUse::Spin);
            frame.render_stateful_widget(throb, throbber_frame, &mut self.throbber.borrow_mut());
        } else {
            frame.render_widget(
                Line::from(vec![if failed {
                    "❌ ".white()
                } else {
                    "🎉 ".white()
                }])
                .left_aligned(),
                throbber_frame,
            );
        }

        if let Some(time_taken) = time_taken {
            if !failed {
                frame.render_widget(
                    Line::from(vec![format!("{:.1}s", time_taken.as_secs_f32()).dark_gray()])
                        .left_aligned(),
                    time_frame,
                );
            }
        }
    }

    fn render_stats(&self, frame: &mut Frame<'_>, area: Rect, state: RenderState) {
        let [current_platform, app_features, serve_address]: [_; 3] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .areas(area);

        frame.render_widget(
            Paragraph::new(Line::from(vec![
                "Platform: ".gray(),
                self.platform.expected_name().yellow(),
                if state.opts.build_arguments.fullstack {
                    " + fullstack".yellow()
                } else {
                    " ".dark_gray()
                },
            ]))
            .wrap(Wrap { trim: false }),
            current_platform,
        );

        self.render_feature_list(frame, app_features, state);

        // todo(jon) should we write https ?
        let address = match state.server.server_address() {
            Some(address) => format!("http://{}", address).blue(),
            None => "no server address".dark_gray(),
        };

        frame.render_widget_ref(
            Paragraph::new(Line::from(vec![
                if self.platform == Platform::Web {
                    "Serving at: ".gray()
                } else {
                    "ServerFns at: ".gray()
                },
                address,
            ]))
            .wrap(Wrap { trim: false }),
            serve_address,
        );
    }

    fn render_feature_list(&self, frame: &mut Frame<'_>, area: Rect, state: RenderState) {
        frame.render_widget(
            Paragraph::new(Line::from({
                let mut lines = vec!["App features: ".gray(), "[".yellow()];

                let feature_list: Vec<String> = state.build_engine.request.all_target_features();
                let num_features = feature_list.len();

                for (idx, feature) in feature_list.into_iter().enumerate() {
                    lines.push("\"".yellow());
                    lines.push(feature.yellow());
                    lines.push("\"".yellow());
                    if idx != num_features - 1 {
                        lines.push(", ".dark_gray());
                    }
                }

                lines.push("]".yellow());

                lines
            }))
            .wrap(Wrap { trim: false }),
            area,
        );
    }

    fn render_more_modal(&self, frame: &mut Frame<'_>, area: Rect, _state: RenderState) {
        let [top, bottom] = Layout::vertical([Constraint::Fill(1), Constraint::Length(2)])
            .horizontal_margin(1)
            .areas(area);

        let meta_list: [_; 5] = Layout::vertical([
            Constraint::Length(1), // spacing
            Constraint::Length(1), // item 1
            Constraint::Length(1), // item 2
            Constraint::Length(1), // item 3
            Constraint::Length(1), // Spacing
        ])
        .areas(top);

        frame.render_widget(
            Paragraph::new(Line::from(vec![
                "dx version: ".gray(),
                self.dx_version.as_str().yellow(),
            ])),
            meta_list[1],
        );
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                "rustc: ".gray(),
                "1.79.9 (nightly)".yellow(),
            ])),
            meta_list[2],
        );
        frame.render_widget(
            Paragraph::new(Line::from(vec!["Hotreload: ".gray(), "rsx only".yellow()])),
            meta_list[3],
        );

        let links_list: [_; 2] =
            Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).areas(bottom);

        frame.render_widget(
            Paragraph::new(Line::from(vec![
                "Read the docs: ".gray(),
                "https://dioxuslabs.com/0.6/docs".blue(),
            ])),
            links_list[0],
        );

        frame.render_widget(
            Paragraph::new(Line::from(vec![
                "Video tutorials: ".gray(),
                "https://youtube.com/@DioxusLabs".blue(),
            ])),
            links_list[1],
        );
    }

    /// Render the version number on the bottom right
    fn render_bottom_row(&self, frame: &mut Frame, area: Rect, _state: RenderState) {
        // Split the area into two chunks
        let row = Layout::horizontal([Constraint::Fill(1), Constraint::Fill(1)]).split(area);

        frame.render_widget(
            Paragraph::new(Line::from(vec![
                // "🧬 dx".dark_gray(),
                // " ".dark_gray(),
                // self.dx_version.as_str().dark_gray(),
                // " ".dark_gray(),
            ]))
            .right_aligned(),
            row[1],
        );
    }

    /// Render borders around the terminal, forcing an inner clear while we're at it
    fn render_borders(&self, frame: &mut Frame, area: Rect) {
        frame.render_widget(ratatui::widgets::Clear, area);
        frame.render_widget(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::DarkGray)),
            area,
        );
    }

    /// Print logs to the terminal as close to a regular "println!()" as possible.
    ///
    /// We don't want alternate screens or other terminal tricks because we want these logs to be as
    /// close to real as possible. Once the log is printed, it is lost, so we need to be very careful
    /// here to not print it incorrectly.
    ///
    /// This method works by printing lines at the top of the viewport frame, and then scrolling up
    /// the viewport accordingly, such that our final call to "clear"  will cause the terminal the viewport
    /// to be comlpetely erased and rewritten. This is slower since we're going around ratatui's diff
    /// logic, but it's the only way to do this that gives us "true println!" semantics.
    ///
    /// In the future, Ratatui's insert_before method will get scroll regions, which will make this logic
    /// much simpler. In that future, we'll simply insert a line into the scrollregion which should automatically
    /// force that portion of the terminal to scroll up.
    ///
    /// TODO(jon): we could look into implementing scroll regions ourselves, but I think insert_before will
    /// land in a reasonable amount of time.
    #[deny(clippy::manual_saturating_arithmetic)]
    fn drain_logs(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    ) -> io::Result<()> {
        use unicode_segmentation::UnicodeSegmentation;

        let Some(log) = self.pending_logs.pop_back() else {
            return Ok(());
        };

        // Only show debug logs if verbose is enabled
        if log.level == Level::DEBUG && !self.verbose {
            return Ok(());
        }

        if log.level == Level::TRACE && !self.trace {
            return Ok(());
        }

        // Grab out the size and location of the terminal and its viewport before we start messing with it
        let frame_rect = terminal.get_frame().area();
        let term_size = terminal.size().unwrap();

        // Render the log into an ansi string
        // We're going to add some metadata to it like the timestamp and source and then dump it to the raw ansi sequences we need to send to crossterm
        let output_sequence = tracemsg_to_ansi_string(log, term_size.width);

        // Get the lines of the output sequence and their overflow
        let lines = output_sequence.lines().collect::<Vec<_>>();
        let lines_printed = lines
            .iter()
            .map(|line| {
                // Very important to strip ansi codes before counting graphemes - the ansi codes count as multiple graphemes!
                let grapheme_count = console::strip_ansi_codes(line).graphemes(true).count() as u16;
                grapheme_count.max(1).div_ceil(term_size.width)
            })
            .sum::<u16>();

        // The viewport might be clipped, but the math still needs to work out.
        let actual_vh_height = self.viewport_current_height().min(term_size.height);

        // We don't need to add any pushback if the frame is in the middle of the viewport
        // We'll then add some pushback to ensure the log scrolls up above the viewport.
        let max_scrollback = lines_printed.min(actual_vh_height.saturating_sub(1));

        // Move the terminal's cursor down to the number of lines printed
        let remaining_space = term_size
            .height
            .saturating_sub(frame_rect.y + frame_rect.height);

        // Calculate how many lines we need to push back
        let to_push = max_scrollback.saturating_sub(remaining_space + 1);

        // Wipe the viewport clean so it doesn't tear
        crossterm::queue!(
            std::io::stdout(),
            crossterm::cursor::MoveTo(0, frame_rect.y),
            crossterm::terminal::Clear(ClearType::FromCursorDown),
        )?;

        // The only reliable way we can force the terminal downards is through "insert_before"
        // If we need to push the terminal down, we'll use this method with the number of lines
        // Ratatui will handle this rest.
        // FIXME(jon): eventually insert_before will get scroll regions, breaking this, but making the logic here simpler
        if to_push == 0 {
            terminal.insert_before(lines_printed.saturating_sub(1), |_| {})?;
        }

        // Start printing the log by writing on top of the topmost line
        for (idx, line) in lines.into_iter().enumerate() {
            // Move the cursor to the correct line offset but don't go past the bottom of the terminal
            let start = frame_rect.y + idx as u16;
            let start = start.min(term_size.height - 1);
            crossterm::queue!(
                std::io::stdout(),
                crossterm::cursor::MoveTo(0, start),
                crossterm::style::Print(line),
                crossterm::style::Print("\n"),
            )?;
        }

        // Scroll the terminal if we need to
        for _ in 0..to_push {
            crossterm::queue!(
                std::io::stdout(),
                crossterm::cursor::MoveTo(0, term_size.height - 1),
                crossterm::style::Print("\n"),
            )?;
        }

        // Force a clear
        // Might've been triggered by insert_before already, but supposedly double-queuing is fine
        // since this isn't a "real" synchronous clear
        terminal.clear()?;

        Ok(())
    }

    fn viewport_current_height(&self) -> u16 {
        match self.more_modal_open {
            true => VIEWPORT_HEIGHT_BIG,
            false => VIEWPORT_HEIGHT_SMALL,
        }
    }
}

fn tracemsg_to_ansi_string(log: TraceMsg, term_width: u16) -> String {
    let rendered = match log.content {
        TraceContent::Cargo(msg) => msg.message.rendered.unwrap_or_default(),
        TraceContent::Text(text) => text,
    };

    // Create a paragraph widget using the log line itself
    // From here on out, we want to work with the escaped ansi string and the "real lines" to be printed
    //
    // We make a special case for lines that look like frames (ie ==== or ---- or ------) and make them
    // dark gray, just for readability.
    //
    // todo(jon): refactor this out to accept any widget, not just paragraphs
    let paragraph = Paragraph::new({
        use ansi_to_tui::IntoText;
        use chrono::Timelike;

        let mut text = Text::default();

        for (idx, raw_line) in rendered.lines().enumerate() {
            let line_as_text = raw_line.into_text().unwrap();
            let is_pretending_to_be_frame = raw_line
                .chars()
                .all(|c| c == '=' || c == '-' || c == ' ' || c == '─');

            for (subline_idx, line) in line_as_text.lines.into_iter().enumerate() {
                let mut out_line = if idx == 0 && subline_idx == 0 {
                    let mut formatted_line = Line::default();

                    formatted_line.push_span(
                        Span::raw(format!(
                            "{:02}:{:02}:{:02} ",
                            log.timestamp.hour(),
                            log.timestamp.minute(),
                            log.timestamp.second()
                        ))
                        .dark_gray(),
                    );
                    formatted_line.push_span(
                        Span::raw(format!(
                            "[{src}] {padding}",
                            src = log.source,
                            padding =
                                " ".repeat(3usize.saturating_sub(log.source.to_string().len()))
                        ))
                        .style(match log.source {
                            TraceSrc::App(_platform) => Style::new().blue(),
                            TraceSrc::Dev => Style::new().magenta(),
                            TraceSrc::Build => Style::new().yellow(),
                            TraceSrc::Bundle => Style::new().magenta(),
                            TraceSrc::Cargo => Style::new().yellow(),
                            TraceSrc::Unknown => Style::new().gray(),
                        }),
                    );

                    for span in line.spans {
                        formatted_line.push_span(span);
                    }

                    formatted_line
                } else {
                    line
                };

                if is_pretending_to_be_frame {
                    out_line = out_line.dark_gray();
                }

                text.lines.push(out_line);
            }
        }

        text
    });

    // We want to get the escaped ansii string and then by dumping the paragraph as ascii codes (again)
    //
    // This is important because the line_count method on paragraph takes into account the width of these codes
    // the 3000 clip width is to bound log lines to a reasonable memory usage
    // We could consider reusing this buffer since it's a lot to allocate, but log printing is not the
    // slowest thing in the world and allocating is pretty fast...
    //
    // After we've dumped the ascii out, we want to call "trim_end" which ensures we don't attempt
    // to print extra characters as lines, since AnsiStringBuffer will in fact attempt to print empty
    // cells as characters. That might not actually be important, but we want to shrink the buffer
    // before printing it
    let line_count = paragraph.line_count(term_width);
    AnsiStringBuffer::new(3000, line_count as u16).render(&paragraph)
}
