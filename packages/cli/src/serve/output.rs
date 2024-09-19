use crate::config::AddressArguments;
use crate::{
    builder::{BuildMessage, BuildUpdateProgress, Stage, UpdateStage},
    cli::serve::ServeArgs,
    dioxus_crate::DioxusCrate,
    serve::{Builder, Watcher},
    tracer::CLILogControl,
    Platform, TraceMsg, TraceSrc,
};
use ansi_to_tui::IntoText;
use crossterm::{
    cursor::{Hide, Show},
    event::{
        DisableBracketedPaste, DisableFocusChange, DisableMouseCapture, EnableBracketedPaste,
        EnableFocusChange, EnableMouseCapture, Event, EventStream, KeyCode, KeyEvent, KeyEventKind,
        KeyModifiers,
    },
    execute,
    terminal::{
        disable_raw_mode, enable_raw_mode, Clear, ClearType, EnterAlternateScreen,
        LeaveAlternateScreen,
    },
    tty::IsTty,
    ExecutableCommand,
};
use dioxus_devtools_types::ClientMsg;
use futures_util::{
    future::{select_all, OptionFuture},
    Future, FutureExt,
};
use ratatui::{
    prelude::*,
    widgets::{Block, BorderType, Borders, Gauge, LineGauge, Paragraph, WidgetRef, Wrap},
    TerminalOptions, Viewport,
};
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    fmt::Display,
    io::{self, stdout},
    ops::Add,
    rc::Rc,
    time::{Duration, Instant},
};

use super::{ansi_buffer::AnsiStringBuffer, loggs::*};
use super::{AppHandle, DevServer, ServeUpdate};
use tracing::Level;

use super::render;

// How many lines should be scroll on each mouse scroll or arrow key input.
const SCROLL_SPEED: u16 = 2;
// Speed added to `SCROLL_SPEED` when the modifier key is held during scroll.
const SCROLL_MODIFIER: u16 = 4;
// Scroll modifier key.
const SCROLL_MODIFIER_KEY: KeyModifiers = KeyModifiers::SHIFT;

const VIEWPORT_WIDTH: u16 = 120;
const VIEWPORT_HEIGHT_SMALL: u16 = 7;
const VIEWPORT_HEIGHT_BIG: u16 = 14;

pub struct Output {
    term: Rc<RefCell<Option<Terminal<CrosstermBackend<io::Stdout>>>>>,
    events: Option<EventStream>,

    pub(crate) build_progress: BuildProgress,

    // A list of all messages from build, dev, app, and more.
    messages: Vec<ConsoleMessage>,
    more_modal_open: bool,
    anim_start: Instant,
    interactive: bool,
    platform: Platform,

    // Pending logs
    pending_logs: Vec<TraceMsg>,

    // Filters
    show_filter_menu: bool,
    filters: Vec<(String, bool)>,
    selected_filter_index: usize,
    filter_search_mode: bool,
    filter_search_input: Option<String>,

    dx_version: String,
    throbber: RefCell<throbber_widgets_tui::ThrobberState>,
    progress: f64,
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
            messages: Vec::new(),
            more_modal_open: false,
            build_progress: Default::default(),
            anim_start: Instant::now(),
            pending_logs: Vec::new(),

            // Filter
            show_filter_menu: false,
            filters: Vec::new(),
            selected_filter_index: 0,
            filter_search_input: None,
            filter_search_mode: false,

            // Status bars
            throbber: RefCell::new(throbber_widgets_tui::ThrobberState::default()),
            progress: 0.0,
        };

        output.startup()?;

        Ok(output)
    }

    fn startup(&mut self) -> io::Result<()> {
        // set the panic hook to fix the terminal
        set_fix_term_hook();

        if self.interactive {
            enable_raw_mode()?;
            stdout()
                .execute(Hide)?
                .execute(EnableFocusChange)?
                .execute(EnableBracketedPaste)?;

            // workaround for ci where the terminal is not fully initialized
            // https://github.com/crossterm-rs/crossterm/issues/659
            self.events = Some(EventStream::new());
        }

        Ok(())
    }

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

    /// Add a message from stderr to the logs
    pub fn push_stderr(&mut self, platform: Platform, stderr: String) {
        self.messages.push(ConsoleMessage::Log(TraceMsg {
            source: TraceSrc::App(platform),
            level: Level::ERROR,
            content: stderr,
        }));
    }

    /// Add a message from stdout to the logs
    pub fn push_stdout(&mut self, platform: Platform, stdout: String) {
        self.messages.push(ConsoleMessage::Log(TraceMsg {
            source: TraceSrc::App(platform),
            level: Level::INFO,
            content: stdout,
        }));
    }

    pub(crate) async fn wait(&mut self) -> ServeUpdate {
        use futures_util::StreamExt;

        loop {
            let next = OptionFuture::from(self.events.as_mut().map(|f| f.next()));
            let event = tokio::select! {
                Some(Some(Ok(event))) = next => event,
                else => futures_util::future::pending().await
            };

            match self.handle_input(event) {
                Ok(Some(update)) => return update,
                Err(ee) => {
                    return ServeUpdate::Exit {
                        error: Some(Box::new(ee)),
                    }
                }
                _ => (),
            }
        }
    }

    /// Handle an input event, returning `true` if the event should cause the program to restart.
    pub(crate) fn handle_input(&mut self, input: Event) -> io::Result<Option<ServeUpdate>> {
        // handle ctrlc
        if let Event::Key(key) = input {
            if let KeyCode::Char('c') = key.code {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    return Ok(Some(ServeUpdate::Exit { error: None }));
                }
            }
        }

        match input {
            Event::Key(key) => self.handle_keypress(key),

            // todo: if the size is smaller, then we want to clear and redraw to not leave garbage
            // will be hard to calculate overflow, but we need to do it
            Event::Resize(_, _) | _ => Ok(Some(ServeUpdate::Redraw)),
        }
    }

    fn handle_keypress(&mut self, key: KeyEvent) -> io::Result<Option<ServeUpdate>> {
        match key.code {
            KeyCode::Char('r') => return Ok(Some(ServeUpdate::RequestRebuild)),
            KeyCode::Char('o') => {
                // Open the running app.
                // open::that(format!("http://{}:{}", self.addr, self.port))?;

                let mut buf = String::new();

                for x in 0..1000 {
                    buf.push_str(format!("hello {x}").as_str());
                }

                tracing::info!("msg! {buf}");
            }
            KeyCode::Char('/') => {
                // Toggle more modal
                self.more_modal_open = !self.more_modal_open;

                let new_size = self.viewport_height();

                let mut term = self.term.borrow_mut();
                let terminal = term.as_mut().unwrap();

                let size = terminal.size().unwrap();
                terminal.resize(Rect::new(
                    0,
                    0,
                    size.width,
                    match self.more_modal_open {
                        true => size.height + 1,
                        false => size.height - 1,
                    },
                ))?;

                *terminal = Terminal::with_options(
                    CrosstermBackend::new(stdout()),
                    TerminalOptions {
                        viewport: Viewport::Inline(new_size),
                    },
                )
                .unwrap();
            }

            _ => {}
        }

        Ok(None)
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

    pub(crate) fn push_inner_log(&mut self, msg: String) {
        self.push_log(TraceMsg::new(TraceSrc::Build, Level::INFO, msg));
        self.throbber.borrow_mut().calc_next();
    }

    pub(crate) fn new_build_logs(&mut self, platform: Platform, update: BuildUpdateProgress) {
        match update.update {
            UpdateStage::Start => {
                // tracing::info!(dx_src = ?TraceSrc::Build, "Starting build for {platform:?}")
            }
            UpdateStage::SetProgress(progress) => {
                self.progress = progress;
            }
            UpdateStage::Failed(err) => {
                // tracing::error!(dx_src = ?TraceSrc::Build, "Build failed for {platform:?}: {err:?}")
            }
            UpdateStage::AddMessage(build_message) => {
                // tracing::info!(dx_src = ?TraceSrc::Build, "{build_message:?}")
            }
        }
    }

    pub fn push_log(&mut self, message: TraceMsg) {
        self.pending_logs.push(message);
    }

    pub(crate) fn new_ready_app(&mut self, handle: &AppHandle) {
        self.progress = 1.0;
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
        if !self.interactive {
            return;
        }

        let owned_term = self.term.clone();
        let mut term = owned_term.borrow_mut();
        if let Some(term) = term.as_mut() {
            _ = self.drain_logs(term);
            _ = term.draw(|frame| self.render_frame(frame));
        }
    }

    fn viewport_height(&self) -> u16 {
        match self.more_modal_open {
            true => VIEWPORT_HEIGHT_BIG,
            false => VIEWPORT_HEIGHT_SMALL,
        }
    }

    fn drain_logs(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    ) -> io::Result<()> {
        use unicode_segmentation::UnicodeSegmentation;

        // todo: wrong direction of pop.... needs to be a deqeue
        let Some(log) = self.pending_logs.pop() else {
            return Ok(());
        };

        // Grab out the size and location of the terminal and its viewport
        let frame_rect = terminal.get_frame().area();
        let term_size = terminal.size().unwrap();

        // Create a paragraph by escaping the contents of the log, which is already ansi escaped
        // let paragraph = log.content.into_text().unwrap();
        // let line_count = paragraph.lines.len() as u16;
        // let line_count = log.content.lines().count() as u16;
        let mut overflowed_lines = 0;
        for line in log.content.lines() {
            let grapheme_count = line.graphemes(true).count() as u16;
            if grapheme_count > term_size.width {
                // overflowed_lines += 1;
                // overflowed_lines += (grapheme_count / term_size.width) - 1;
                overflowed_lines += (grapheme_count.div_ceil(term_size.width) - 1);
            }
        }

        let byte_count = log.content.len() as u16;
        let paragraph = Paragraph::new(log.content.into_text().unwrap());
        let line_count = paragraph.line_count(term_size.width) as u16;

        // We want to get the escaped ansii string and then by dumping the paragraph as ascii codes (again)
        // This is important because the line_count method on paragraph takes into account the width of these codes
        let mut raw_ansi_buf = AnsiStringBuffer::new(3000.max(byte_count), line_count);
        raw_ansi_buf.render_ref(&paragraph, raw_ansi_buf.buf.area);

        // Calculate how many lines we need to draw by adding the real lines to the wrapped lines
        let lines_to_draw = line_count + overflowed_lines;

        let space_available = term_size.height - self.viewport_height() - 1;

        // Rendering this line will eat the frame, so just shortcut a more reliable path
        // Render the new line at the top of the viewport, and then some spaces so that when we call "clear"
        // The lines will have been scrolled up
        if space_available < lines_to_draw {
            crossterm::queue!(
                std::io::stdout(),
                crossterm::cursor::MoveTo(0, frame_rect.y),
                crossterm::style::Print(raw_ansi_buf.to_string().trim_end()),
                crossterm::style::Print("\n"),
                crossterm::style::Print(
                    (0..self.viewport_height() - 1)
                        .map(|_| "\n")
                        .collect::<String>()
                ),
            )?;
            terminal.clear()?;
            return Ok(());
        }

        // In the case where the log will fit on the screen, we want to make some room for it
        // by adding some lines above the viewport. `insert_before` will eventually use scroll regions
        // in ratatui, so we're just going to use that, even if it has extra flickering in the interim.
        terminal.insert_before(lines_to_draw, |_| {})?;

        // If the viewport is at the bottom of the screen, our new log will be inserted right above
        // the viewport. If not, the viewport will shift down by lines_to_draw *or* the space available
        let y_offset = match frame_rect.y - 1 < space_available {
            true => 0,
            false => lines_to_draw,
        };

        // Finally, print the log to the terminal using crossterm, not ratatui
        // We are careful to handle the case where the log won't fit on the screen, since that will
        // cause this code to be called with the wrong viewport and cause tearing.
        raw_ansi_buf.trim_end();
        let buf = raw_ansi_buf.to_string();

        let mut max_idx = 0_u16;
        for (idx, line) in buf.lines().enumerate() {
            if line.is_empty() {
                continue;
            }
            let start = frame_rect.y.saturating_sub(y_offset) + idx as u16;
            crossterm::queue!(
                std::io::stdout(),
                crossterm::cursor::MoveTo(0, start),
                crossterm::style::Print(line),
                crossterm::style::Print("\n"),
            );
            max_idx = idx as _;
        }

        if (max_idx - line_count) != 0 {
            panic!("\n\n\n\nmax_idx: {max_idx}, line_count: {line_count}, overflowed_lines: {overflowed_lines}");
        }

        Ok(())
    }

    fn render_frame(&self, frame: &mut Frame) {
        let mut size = frame.size();
        size.width = size.width.min(VIEWPORT_WIDTH);

        // stroke the outer border first
        self.render_borders(frame, size);

        // And then start splitting up the frame into chunks
        // First chunk is the entire block itself
        let body = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Fill(1),
                Constraint::Length(1),
            ])
            .horizontal_margin(1)
            .split(size);

        self.render_top_row(frame, body[0]);
        self.render_body(frame, body[1]);
        self.render_bottom_row(frame, body[2]);
    }

    /// Render all decorations.
    pub fn render_borders(&self, frame: &mut Frame, area: Rect) {
        frame.render_widget(ratatui::widgets::Clear, area);
        frame.render_widget(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::DarkGray)),
            area,
        );
    }

    /// Render the status bar.
    pub fn render_bottom_row(&self, frame: &mut Frame, area: Rect) {
        let _platform: Platform = self.platform;
        let build_progress: &BuildProgress = &self.build_progress;
        let more_modal_open: bool = self.more_modal_open;
        let filter_menu_open: bool = self.show_filter_menu;
        let dx_version: &str = &self.dx_version;

        // left aligned text
        let mut left_line = Line::from(vec![
            Span::from("🧬 dx").dark_gray(),
            Span::from(" ").dark_gray(),
            Span::from(dx_version).dark_gray(),
        ]);

        // // If there is build progress, render the current status.
        // let is_build_progress = !build_progress.current_builds.is_empty();
        // if is_build_progress {
        //     // If the build failed, show a failed status.
        //     // Otherwise, render current status.
        //     let build_failed = build_progress
        //         .current_builds
        //         .values()
        //         .any(|b| b.failed.is_some());

        //     if build_failed {
        //         spans.push(Span::from("Build failed ❌").red());
        //     } else {
        //         let build = build_progress
        //             .current_builds
        //             .values()
        //             .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        //             .unwrap();
        //         spans.extend_from_slice(&build.make_spans(Rect::new(
        //             0,
        //             0,
        //             build.max_layout_size(),
        //             1,
        //         )));
        //     }
        // }

        // let filter_span = Span::from("[f] filter");
        // let filter_span = match filter_menu_open {
        //     true => filter_span.light_yellow(),
        //     false => filter_span.gray(),
        // };

        let more_span = Span::from("[/] more");
        let more_span = match more_modal_open {
            true => more_span.light_yellow(),
            false => more_span.gray(),
        };

        let right_line = Line::from(vec![
            Span::from("[o] open").gray(),
            Span::from(" | ").gray(),
            Span::from("[r] rebuild").gray(),
            Span::from(" | ").gray(),
            more_span,
        ]);

        // Split the area into two chunks
        let row = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Fill(1), Constraint::Fill(1)])
            .split(area);

        // frame.render_widget(Paragraph::new(right_line).left_aligned(), row[0]);
        frame.render_widget(Paragraph::new(left_line).right_aligned(), row[1]);
        // frame.render_widget(Paragraph::new(left_line).left_aligned(), row[0]);
        // frame.render_widget(Paragraph::new(right_line).right_aligned(), row[1]);
    }

    fn render_top_row(&self, frame: &mut Frame<'_>, area: Rect) {
        // right aligned text
        let more_span = Span::from("[/] more");
        let more_span = match self.more_modal_open {
            true => more_span.light_yellow(),
            false => more_span.gray(),
        };

        // Right-aligned text
        let right_line = Line::from(vec![
            // Span::from("[o] open").gray(),
            // Span::from(" | ").gray(),
            Span::from("[r] rebuild").gray(),
            Span::from(" | ").gray(),
            more_span,
        ]);

        let row = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Fill(1), Constraint::Fill(1)])
            .split(area);

        // frame.render_widget(Paragraph::new(right_line).right_aligned(), row[1]);
    }

    fn render_body(&self, frame: &mut Frame<'_>, area: Rect) {
        let [title, body, more, foot] = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2),
                Constraint::Length(3),
                Constraint::Fill(1),
                Constraint::Length(0),
            ])
            .horizontal_margin(1)
            .areas(area);

        let [col1, col2] = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Fill(1), Constraint::Fill(1)])
            .horizontal_margin(1)
            .areas(body);

        self.render_body_title(frame, title);
        self.render_gauges(frame, col1);
        self.render_stats(frame, col2);

        if self.more_modal_open {
            self.render_more_modal(frame, more);
        }
    }

    fn render_gauges(&self, frame: &mut Frame<'_>, area: Rect) {
        let [gauge_area, _margin] = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Fill(1), Constraint::Length(5)])
            .areas(area);

        let gauge_list: [_; 3] = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // g1
                Constraint::Length(1), // g2
                Constraint::Length(1), // g3
            ])
            .areas(gauge_area);

        self.render_single_gauge(
            frame,
            gauge_list[0],
            (self.progress * 3.0) - 0.0,
            "Compiling  ",
        );
        self.render_single_gauge(
            frame,
            gauge_list[1],
            (self.progress * 3.0) - 1.0,
            "Optimizing ",
        );
        self.render_single_gauge(
            frame,
            gauge_list[2],
            (self.progress * 3.0) - 2.0,
            "Bundling   ",
        );
    }

    fn render_single_gauge(&self, frame: &mut Frame<'_>, area: Rect, value: f64, label: &str) {
        let value = value.max(0.0).min(1.0);

        let [gauge_row, _, icon] = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Fill(1),
                Constraint::Length(2),
                Constraint::Length(2),
            ])
            .areas(area);

        frame.render_widget(
            LineGauge::default()
                .filled_style(Style::default().fg(match value {
                    1.0 => Color::Green,
                    _ => Color::Yellow,
                }))
                .unfilled_style(Style::default().fg(Color::DarkGray))
                .label(label.gray())
                .line_set(symbols::line::THICK)
                .ratio(value),
            gauge_row,
        );

        if value != 1.0 {
            let throb = throbber_widgets_tui::Throbber::default()
                .style(ratatui::style::Style::default().fg(ratatui::style::Color::Cyan))
                .throbber_style(
                    ratatui::style::Style::default()
                        .fg(ratatui::style::Color::Red)
                        .add_modifier(ratatui::style::Modifier::BOLD),
                )
                .throbber_set(throbber_widgets_tui::CLOCK)
                .use_type(throbber_widgets_tui::WhichUse::Spin);
            frame.render_stateful_widget(throb, icon, &mut self.throbber.borrow_mut());
        } else {
            frame.render_widget(Line::from("🎉".white()).right_aligned(), icon);
        }
    }

    fn render_stats(&self, frame: &mut Frame<'_>, area: Rect) {
        let stat_list: [_; 3] = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
            .areas(area);

        let s1 = Paragraph::new(Line::from(vec![
            "Serving at ".gray(),
            "http://127.0.0.1:8080".blue(),
        ]))
        .wrap(Wrap { trim: false });
        let s2 = Paragraph::new(Line::from(vec![
            "Platform: ".gray(),
            "web".yellow(),
            " + ".gray(),
            "fullstack".yellow(),
        ]))
        .wrap(Wrap { trim: false });
        let s3 = Paragraph::new(Line::from(vec!["Build time: ".gray(), "1m 2s".yellow()]))
            .wrap(Wrap { trim: false });

        frame.render_widget_ref(s1, stat_list[0]);
        frame.render_widget(s2, stat_list[1]);
        frame.render_widget(s3, stat_list[2]);
    }

    fn render_body_title(&self, frame: &mut Frame<'_>, title: Rect) {
        // Right-aligned text
        let right_line = Line::from(vec![
            Span::from("[o] open").gray(),
            Span::from(" | ").gray(),
            Span::from("[r] rebuild").gray(),
            Span::from(" | ").gray(),
            match self.more_modal_open {
                true => Span::from("[/] more").light_yellow(),
                false => Span::from("[/] more").gray(),
            },
        ]);

        // // Split the area into two chunks
        // let row = Layout::default()
        //     .direction(Direction::Horizontal)
        //     .constraints([Constraint::Fill(1), Constraint::Fill(1)])
        //     .split(area);

        // // frame.render_widget(Paragraph::new(right_line).left_aligned(), row[0]);
        // frame.render_widget(Paragraph::new(left_line).right_aligned(), row[1]);

        frame.render_widget(
            Paragraph::new(Line::from(vec![
                "Serving ".yellow(),
                "your dioxus app: ".white(),
                "file-explorer".light_blue(),
                "! 🚀".white(),
            ]))
            .wrap(Wrap { trim: false })
            .left_aligned(),
            title,
        );

        frame.render_widget(right_line.right_aligned(), title);
    }

    fn render_more_modal(&self, frame: &mut Frame<'_>, area: Rect) {
        let [top, bottom] = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Fill(1), Constraint::Length(2)])
            .horizontal_margin(1)
            .areas(area);

        let meta_list: [_; 5] = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // spacing
                Constraint::Length(1), // item 1
                Constraint::Length(1), // item 2
                Constraint::Length(1), // item 3
                Constraint::Length(1), // Spacing
            ])
            .areas(top);

        frame.render_widget(
            Paragraph::new(Line::from(vec![
                "Watching: ".dark_gray(),
                r#"[“assets”, “src”]"#.yellow(),
            ])),
            meta_list[1],
        );
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                "rustc: ".dark_gray(),
                "1.79.9 (nightly)".yellow(),
            ])),
            meta_list[2],
        );
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                "Hotreload: ".dark_gray(),
                "enabled".yellow(),
            ])),
            meta_list[3],
        );

        let links_list: [_; 2] = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Length(1)])
            .areas(bottom);

        frame.render_widget(
            Paragraph::new(Line::from(vec![
                "Read the docs: ".dark_gray(),
                "https://dioxuslabs.com/0.6/docs".blue(),
            ])),
            links_list[0],
        );

        frame.render_widget(
            Paragraph::new(Line::from(vec![
                "Video tutorials: ".dark_gray(),
                "https://youtube.com/dioxuslabs".blue(),
            ])),
            links_list[1],
        );
    }

    // /// Renders the "more" modal to show extra info/keybinds accessible via the more keybind.
    // pub fn render_more_modal(&self, frame: &mut Frame) {
    //     let modal = Layout::default()
    //         .direction(Direction::Vertical)
    //         .constraints([Constraint::Fill(1), Constraint::Length(5)])
    //         .split(self.console[0])[1];

    //     frame.render_widget(ratatui::widgets::Clear, modal);
    //     frame.render_widget(Block::default().borders(Borders::ALL), modal);

    //     // Render under construction message
    //     frame.render_widget(
    //         Paragraph::new("Under construction, please check back at a later date!")
    //             .alignment(Alignment::Center),
    //         modal,
    //     );
    // }
}
