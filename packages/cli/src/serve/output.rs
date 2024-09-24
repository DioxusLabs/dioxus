use crate::{
    serve::{ansi_buffer::AnsiStringBuffer, Builder, DevServer, ServeUpdate, Watcher},
    BuildStage, BuildUpdate, DioxusCrate, Platform, ServeArgs, TraceMsg, TraceSrc,
};
use crossterm::{
    cursor::{Hide, Show},
    event::{
        DisableBracketedPaste, DisableFocusChange, EnableBracketedPaste, EnableFocusChange, Event,
        EventStream, KeyCode, KeyEvent, KeyEventKind, KeyModifiers,
    },
    terminal::{disable_raw_mode, enable_raw_mode, ClearType},
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
const VIEWPORT_WIDTH: u16 = 120;
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
    // messages: Vec<TraceMsg>,
    more_modal_open: bool,
    interactive: bool,
    platform: Platform,

    // Whether to show verbose logs or not
    // We automatically hide "debug" logs if verbose is false (only showing "info" / "warn" / "error")
    verbose: bool,

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
    server: &'a DevServer,
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
            dx_version: format!("{}", env!("CARGO_PKG_VERSION")),
            platform: cfg.build_arguments.platform.expect("To be resolved by now"),
            events: None,
            // messages: Vec::new(),
            more_modal_open: false,
            pending_logs: VecDeque::new(),
            throbber: RefCell::new(throbber_widgets_tui::ThrobberState::default()),
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
            Event::Resize(_, _) | _ => Ok(Some(ServeUpdate::Redraw)),
        }
    }

    fn handle_keypress(&mut self, key: KeyEvent) -> io::Result<Option<ServeUpdate>> {
        match key.code {
            KeyCode::Char('r') => return Ok(Some(ServeUpdate::RequestRebuild)),
            KeyCode::Char('o') => return Ok(Some(ServeUpdate::OpenApp)),
            KeyCode::Char('v') => {
                self.verbose = !self.verbose;
                tracing::info!(
                    "Verbose logging is now {}",
                    if self.verbose { "on" } else { "off" }
                );
            }
            KeyCode::Char('c') => {
                tracing::info!("clearing terminal (todo... not working)");
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

    /// Add a message from stderr to the logs
    /// This will queue the stderr message as a TraceMsg and print it on the next render
    /// We'll use the `App` TraceSrc for the msg, and whatever level is provided
    pub fn push_stdio(&mut self, platform: Platform, msg: String, level: Level) {
        self.push_log(TraceMsg::new(TraceSrc::App(platform), level, msg));
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
        self.push_log(TraceMsg::new(TraceSrc::App(platform), level, content));
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
            BuildUpdate::Message {} => {}
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
        server: &DevServer,
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
        area.width = area.width.clamp(0, VIEWPORT_WIDTH);

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

    fn render_body(&self, frame: &mut Frame<'_>, area: Rect, state: RenderState) {
        let [_title, body, more, _foot] = Layout::vertical([
            Constraint::Length(0),
            Constraint::Length(3),
            Constraint::Fill(1),
            Constraint::Length(0),
        ])
        .horizontal_margin(1)
        .areas(area);

        let [col1, col2] = Layout::horizontal([Constraint::Length(60), Constraint::Fill(1)])
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

        let gauge_list: [_; 3] = Layout::vertical([
            Constraint::Length(1), // g1
            Constraint::Length(1), // g2
            Constraint::Length(1), // g3
        ])
        .areas(gauge_area);

        self.render_single_gauge(
            frame,
            gauge_list[0],
            state.build_engine.compile_progress,
            "Compiling  ",
            state,
        );
        self.render_single_gauge(
            frame,
            gauge_list[1],
            state.build_engine.bundling_progress,
            "Bundling   ",
            state,
        );

        let mut lines = vec!["Status:     ".white()];
        match &state.build_engine.stage {
            BuildStage::Initializing => lines.push("Initializing".yellow()),
            BuildStage::InstallingTooling {} => lines.push("Installing tooling".yellow()),
            BuildStage::Compiling {
                current,
                total,
                krate,
            } => {
                lines.push("Compiling ".yellow());
                lines.push(format!("{current}/{total} ").gray());
                lines.push(krate.as_str().dark_gray())
            }
            BuildStage::OptimizingWasm {} => {
                lines.push("Optimizing wasm".yellow());
            }
            BuildStage::OptimizingAssets {} => lines.push("Optimizing assets".yellow()),
            BuildStage::CopyingAssets {
                current,
                total,
                path,
            } => {
                lines.push(format!("Copying asset ").yellow());
                lines.push(format!("{current}/{total} ").gray());
                if let Some(name) = path.file_name().and_then(|f| f.to_str()) {
                    lines.push(name.dark_gray())
                }
            }
            BuildStage::Success => {
                lines.push("Serving ".yellow());
                lines.push(state.krate.executable_name().dark_gray());
            }
            BuildStage::Failed => lines.push("Failed".red()),
            BuildStage::Aborted => lines.push("Aborted".red()),
            BuildStage::Restarting => lines.push("Restarting".yellow()),
        };

        frame.render_widget(Line::from(lines), gauge_list[2]);
    }

    fn render_single_gauge(
        &self,
        frame: &mut Frame<'_>,
        area: Rect,
        value: f64,
        label: &str,
        state: RenderState,
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
            frame.render_stateful_widget(throb, icon, &mut self.throbber.borrow_mut());
        } else {
            frame.render_widget(
                Line::from(vec![
                    if failed {
                        "❌ ".white()
                    } else {
                        "🎉 ".white()
                    },
                    "100ms".dark_gray(),
                ])
                .left_aligned(),
                icon,
            );
        }
    }

    fn render_stats(&self, frame: &mut Frame<'_>, area: Rect, _state: RenderState) {
        let stat_list: [_; 3] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .areas(area);

        frame.render_widget_ref(
            Paragraph::new(Line::from(vec![
                "Serving at: ".gray(),
                "http://127.0.0.1:8080".blue(),
            ]))
            .wrap(Wrap { trim: false }),
            stat_list[0],
        );
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                "Platform: ".gray(),
                self.platform.to_string().yellow(),
                if _state.opts.build_arguments.fullstack {
                    " (fullstack)".yellow()
                } else {
                    "".white()
                },
            ]))
            .wrap(Wrap { trim: false }),
            stat_list[1],
        );
        frame.render_widget(
            Paragraph::new(Line::from(vec!["Build time: ".gray(), "1m 2s".yellow()]))
                .wrap(Wrap { trim: false }),
            stat_list[2],
        );
    }

    fn render_body_title(&self, frame: &mut Frame<'_>, area: Rect, _state: RenderState) {
        frame.render_widget(
            Line::from(vec![
                // todo: re-enable open + clear
                // "c:clear".gray(),
                // "  ".gray(),
                // "o:open".gray(),
                // "  ".gray(),
                " ".dark_gray(),
                "r:rebuild".dark_gray(),
                " ─ ".dark_gray(),
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
                "Watching: ".gray(),
                r#"[“assets”, “src”]"#.yellow(),
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
            Paragraph::new(Line::from(vec!["Hotreload: ".gray(), "enabled".yellow()])),
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
                "https://youtube.com/dioxuslabs".blue(),
            ])),
            links_list[1],
        );
    }

    /// Render the version number on the bottom right
    fn render_bottom_row(&self, frame: &mut Frame, area: Rect, state: RenderState) {
        // Split the area into two chunks
        let row = Layout::horizontal([Constraint::Fill(1), Constraint::Fill(1)]).split(area);

        frame.render_widget(
            Paragraph::new(Line::from(vec![
                " ".dark_gray(),
                state.krate.executable_name().dark_gray(),
                " ─ ".dark_gray(),
                "🧬 dx".dark_gray(),
                " ".dark_gray(),
                self.dx_version.as_str().dark_gray(),
                " ".dark_gray(),
            ]))
            .right_aligned(),
            row[1],
        );
    }

    /// Render all decorations.
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

        // Grab out the size and location of the terminal and its viewport
        let frame_rect = terminal.get_frame().area();
        let term_size = terminal.size().unwrap();

        // Create a paragraph widget using the log line itself
        // From here on out, we want to work with the escaped ansi string and the "real lines" to be printed
        // todo(jon): refactor this out to accept any widget, not just paragraphs
        let paragraph = Paragraph::new({
            use ansi_to_tui::IntoText;
            use chrono::Timelike;

            // Call `into_text` to convert any ansi sequences to tui spans
            let mut text = log.content.into_text().unwrap();

            // And then add the extra log spans to the first line
            text.lines[0] = {
                let mut line = Line::default();
                line.push_span(
                    Span::raw(format!(
                        "{:02}:{:02}:{:02} ",
                        log.timestamp.hour(),
                        log.timestamp.minute(),
                        log.timestamp.second()
                    ))
                    .dark_gray(),
                );
                line.push_span(
                    Span::raw(format!(
                        "[{src}] {padding}",
                        src = log.source,
                        padding = " ".repeat(3usize.saturating_sub(log.source.to_string().len()))
                    ))
                    .style(match log.source {
                        TraceSrc::App(_platform) => Style::new().blue(),
                        TraceSrc::Dev => Style::new().magenta(),
                        TraceSrc::Build => Style::new().yellow(),
                        TraceSrc::Cargo => Style::new().yellow(),
                        TraceSrc::Unknown => Style::new().gray(),
                        TraceSrc::Hotreload => Style::new().light_yellow(),
                    }),
                );
                line.extend(text.lines[0].iter().cloned());
                line
            };
            text
        });
        let line_count = paragraph.line_count(term_size.width) as u16;

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
        let output_sequence = {
            let mut raw_ansi_buf = AnsiStringBuffer::new(3000, line_count);
            raw_ansi_buf.render_ref(&paragraph, raw_ansi_buf.buf.area);
            raw_ansi_buf.dump()
        };

        // Create a paragraph by escaping the contents of the log, which is already ansi escaped
        let mut overflowed_lines = 0;
        for line in output_sequence.lines() {
            let grapheme_count = line.graphemes(true).count() as u16;
            if grapheme_count > term_size.width {
                // Subtract 1 since we already know this line will count as at least one line
                overflowed_lines += grapheme_count.div_ceil(term_size.width) - 1;
            }
        }

        // Calculate how many lines we need to draw by adding the real lines to the wrapped lines
        let lines_to_draw = line_count + overflowed_lines;

        // The viewport might be clipped, but the math still needs to work out.
        let actual_vh_height = self.viewport_current_height().min(term_size.height);

        // Determine how many free lines are above the viewport that we can print into
        let space_available = term_size.height.saturating_sub(actual_vh_height);

        // Rendering this line will eat the frame, so just shortcut a more reliable path
        // Render the new line at the top of the viewport, and then some spaces so that when we call "clear"
        // The lines will have been scrolled up
        //
        // FIXME(jon): if a line is longer than the terminal width, it will be truncated since we're not
        // advancing by the grapheme_count
        if space_available < lines_to_draw {
            for (idx, line) in output_sequence.lines().enumerate() {
                // Move the cursor to the correct line but don't go past the bottom of the terminal
                let start = frame_rect.y + idx as u16;
                let start = start.min(term_size.height - 1);
                crossterm::queue!(
                    std::io::stdout(),
                    crossterm::cursor::MoveTo(0, start),
                    crossterm::terminal::Clear(ClearType::CurrentLine),
                    crossterm::style::Print(line.trim_end()),
                    crossterm::style::Print("\n"),
                )?;

                let grapheme_count = line.graphemes(true).count() as u16;
                let lines_overflow = grapheme_count.div_ceil(term_size.width) - 1;
                for _ in 0..lines_overflow {
                    crossterm::queue!(
                        std::io::stdout(),
                        crossterm::cursor::MoveTo(0, start + 1),
                        crossterm::terminal::Clear(ClearType::CurrentLine),
                        crossterm::style::Print("\n"),
                    )?;
                }
            }

            // Push the scrollback buffer up the number of lines we need to clear
            // This will either be the viewport height (for multi-line logs that overflow the viewport)
            // or the number of lines we need to clear (for multi-line logs that fit in the viewport)
            //
            // We're subtracting 2 because overflowed lines already shoved the viewport down
            for _ in 0..actual_vh_height.saturating_sub(2).min(lines_to_draw) {
                crossterm::queue!(
                    std::io::stdout(),
                    crossterm::cursor::MoveTo(0, term_size.height - 1),
                    crossterm::style::Print("\n"),
                )?;
            }
            terminal.clear()?;
            return Ok(());
        }

        // In the case where the log will fit on the screen, we want to make some room for it
        // by adding some lines above the viewport. `insert_before` will eventually use scroll regions
        // in ratatui, so we're just going to use that, even if it has extra flickering in the interim.
        terminal.insert_before(lines_to_draw, |_| {})?;

        // Finally, print the log to the terminal using crossterm, not ratatui
        // We are careful to handle the case where the log won't fit on the screen, since that will
        // cause this code to be called with the wrong viewport and cause tearing.
        for (idx, line) in output_sequence.lines().enumerate() {
            // Move the cursor to the correct line but don't go past the bottom of the terminal
            let start = frame_rect.y.saturating_sub(lines_to_draw) + idx as u16;
            let start = start.min(term_size.height - 1);
            crossterm::queue!(
                std::io::stdout(),
                crossterm::cursor::MoveTo(0, start),
                crossterm::style::Print(line.trim_end()),
                crossterm::style::Print("\n"),
            )?;

            let grapheme_count = line.graphemes(true).count() as u16 + 1;
            let lines_overflow = grapheme_count.div_ceil(term_size.width) - 1;
            for _ in 0..lines_overflow {
                crossterm::queue!(
                    std::io::stdout(),
                    crossterm::cursor::MoveTo(0, start + 1),
                    // crossterm::terminal::Clear(ClearType::CurrentLine),
                    crossterm::style::Print("\n"),
                )?;
            }
        }

        Ok(())
    }

    fn viewport_current_height(&self) -> u16 {
        match self.more_modal_open {
            true => VIEWPORT_HEIGHT_BIG,
            false => VIEWPORT_HEIGHT_SMALL,
        }
    }
}

// // todo: re-enable
// #[allow(unused)]
// async fn rustc_version() -> String {
//     tokio::process::Command::new("rustc")
//         .arg("--version")
//         .output()
//         .await
//         .ok()
//         .map(|o| o.stdout)
//         .and_then(|o| {
//             let out = String::from_utf8(o).unwrap();
//             out.split_ascii_whitespace().nth(1).map(|v| v.to_string())
//         })
//         .unwrap_or_else(|| "<unknown>".to_string())
// }
