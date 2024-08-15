//! This module contains functions to render different elements on the TUI frame.
//!
//! The current TUI layout is:
//! ------------
//! -- CONSOLE--
//! ------------
//! ---BORDER---
//! --INFO BAR--
//! ---BORDER---
//! -STATUS BAR-

use super::{BuildProgress, ConsoleHeight, NumLinesWrapping, ScrollPosition, Tab};
use crate::builder::{BuildMessage, MessageSource, MessageType};
use dioxus_cli_config::Platform;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{
        Block, Borders, Clear, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Widget,
        Wrap,
    },
    Frame,
};
use std::rc::Rc;

pub struct TuiLayout {
    /// The console where build logs are displayed.
    console: Rc<[Rect]>,
    // The border that separates the console and info bar.
    border_sep_one: Rect,
    /// The info bar of keybinds, current tab, etc.
    info_bar: Rc<[Rect]>,
    // The border that separates the two bars (info and status).
    border_sep_two: Rect,
    //. The status bar that displays build status, platform, versions, etc.
    status_bar: Rc<[Rect]>,
}

impl TuiLayout {
    pub fn new(frame_size: Rect) -> Self {
        // The full layout
        let body = Layout::default()
            .direction(Direction::Vertical)
            .constraints(&[
                // Body
                Constraint::Min(0),
                // Border Seperator
                Constraint::Length(1),
                // Footer Keybinds
                Constraint::Length(1),
                // Border Seperator
                Constraint::Length(1),
                // Footer Status
                Constraint::Length(1),
                // Padding
                Constraint::Length(1),
            ])
            .split(frame_size);

        // Build the console, where logs go.
        let console = Layout::default()
            .direction(Direction::Vertical)
            .constraints(&[Constraint::Fill(1)])
            .split(body[0]);

        // Build the info bar for display keybinds.
        let info_bar = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(&[Constraint::Fill(1), Constraint::Fill(1)])
            .split(body[2]);

        // Build the status bar.
        let status_bar = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(&[Constraint::Fill(1)])
            .split(body[4]);

        // Specify borders
        let border_sep_one = body[1];
        let border_sep_two = body[3];

        Self {
            console,
            border_sep_one,
            info_bar,
            border_sep_two,
            status_bar,
        }
    }

    /// Render all  borders.
    pub fn render_borders(&self, frame: &mut Frame) {
        frame.render_widget(Block::new().borders(Borders::TOP), self.border_sep_one);
        frame.render_widget(
            Block::new()
                .borders(Borders::TOP)
                .border_style(Style::new().dark_gray()),
            self.border_sep_two,
        );
    }

    /// Render the console and it's logs.
    pub fn render_console(
        &self,
        frame: &mut Frame,
        scroll: ScrollPosition,
        current_tab: Tab,
        build_progress: &BuildProgress,
        messages: Vec<Message>,
    ) -> NumLinesWrapping {
        // TODO: This doesn't render logs in the order they were created.
        // TODO: Clean and fix this.
        // TODO: We need a single buffer of logs.

        let console = self.console[0];

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
        for platform in build_progress.build_logs.keys() {
            let build = build_progress.build_logs.get(platform).unwrap();

            let msgs = match current_tab {
                Tab::Console => &build.stdout_logs,
                Tab::BuildLog => &build.messages,
            };

            for span in msgs.iter() {
                add_build_message(span);
            }
        }
        // Then log the internal logs
        for message in build_progress.internal_logs.iter() {
            add_build_message(message);
        }

        let paragraph = Paragraph::new(paragraph_text)
            .left_aligned()
            .wrap(Wrap { trim: false });

        let console_height = self.get_console_height();
        let num_lines_wrapping = NumLinesWrapping(paragraph.line_count(console.width) as u16);

        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None)
            .track_symbol(None)
            .thumb_symbol("▐");

        let mut scrollbar_state =
            ScrollbarState::new(num_lines_wrapping.0.saturating_sub(console_height.0) as usize)
                .position(scroll.0 as usize);

        let paragraph = paragraph.scroll((scroll.0, 0));
        paragraph
            .block(Block::new())
            .render(console, frame.buffer_mut());

        // and the scrollbar, those are separate widgets
        frame.render_stateful_widget(
            scrollbar,
            console.inner(Margin {
                // todo: dont use margin - just push down the body based on its top border
                // using an inner vertical margin of 1 unit makes the scrollbar inside the block
                vertical: 1,
                horizontal: 0,
            }),
            &mut scrollbar_state,
        );

        num_lines_wrapping
    }

    /// Render the info bar.
    pub fn render_info_bar(&self, frame: &mut Frame, current_tab: Tab) {
        let mut console_line = Span::from("[1] console");
        let mut build_line = Span::from("[2] build");
        let divider = Span::from("  | ").gray();

        // Display the current tab
        match current_tab {
            Tab::Console => {
                console_line = console_line.fg(Color::LightYellow);
                build_line = build_line.fg(Color::DarkGray);
            }
            Tab::BuildLog => {
                build_line = build_line.fg(Color::LightYellow);
                console_line = console_line.fg(Color::DarkGray);
            }
        }

        // Left-aligned text
        let left_line = Line::from(vec![console_line, divider, build_line]);

        // Right-aligned text
        let right_line = Line::from(vec![
            Span::from("[/] more").dark_gray(),
            Span::from(" | ").gray(),
            Span::from("[r] reload").dark_gray(),
            Span::from(" | ").gray(),
            Span::from("[c] clear").dark_gray(),
            Span::from(" | ").gray(),
            Span::from("[o] open").dark_gray(),
            Span::from(" | ").gray(),
            Span::from("[h] hide").dark_gray(),
        ]);

        // Render the info
        frame.render_widget(Paragraph::new(left_line).left_aligned(), self.info_bar[0]);
        frame.render_widget(Paragraph::new(right_line).right_aligned(), self.info_bar[1]);
    }

    /// Render the status bar.
    pub fn render_status_bar(
        &self,
        frame: &mut Frame,
        is_cli_release: bool,
        platform: Platform,
        build_progress: &BuildProgress,
    ) {
        let mut spans = vec![
            Span::from(if is_cli_release { "dx" } else { "dx-dev" }).green(),
            Span::from(" ").green(),
            Span::from("serve").green(),
            Span::from(" | ").white(),
            Span::from(platform.to_string()).green(),
            Span::from(" | ").white(),
        ];

        // If there is build progress, render the current status.
        let is_build_progress = !build_progress.build_logs.is_empty();
        if is_build_progress {
            // If the build failed, show a failed status.
            // Otherwise, render current status.
            let build_failed = build_progress
                .build_logs
                .values()
                .any(|b| b.failed.is_some());

            if build_failed {
                spans.push(Span::from("build failed ❌").red());
            } else {
                spans.push(Span::from("status: ").green());
                let build = build_progress
                    .build_logs
                    .values()
                    .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                    .unwrap();
                spans.extend_from_slice(&build.spans(Rect::new(0, 0, build.max_layout_size(), 1)));
            }
        }

        frame.render_widget(
            Paragraph::new(Line::from(spans)).left_aligned(),
            self.status_bar[0],
        );
    }

    /// Renders the "more" modal to show extra info/keybinds accessible via the more keybind.
    pub fn render_more_modal(&self, frame: &mut Frame) {
        let area = self.console[0];
        let modal = Layout::default()
            .direction(Direction::Vertical)
            .constraints(&[Constraint::Fill(1)])
            .split(area)[0];

        frame.render_widget(Clear, modal);
        frame.render_widget(Block::default().borders(Borders::ALL), modal);

        // Render under construction message
        let msg = Paragraph::new("Under construction, please check back at a later date!")
            .alignment(Alignment::Center);
        frame.render_widget(msg, modal);
    }

    /// Returns the height of the console TUI area in number of lines.
    pub fn get_console_height(&self) -> ConsoleHeight {
        ConsoleHeight(self.console[0].height)
    }
}
