//! This module contains functions to render different elements on the TUI frame.
//!
//! The current TUI layout is:
//! ------------
//! -- CONSOLE--
//! ------------
//! ---BORDER---
//! -STATUS BAR-

use super::{
    BuildProgress, ConsoleHeight, Message, MessageSource, NumLinesWrapping, ScrollPosition,
};
use ansi_to_tui::IntoText as _;
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
use std::fmt::Write as _;
use std::rc::Rc;
use tracing::Level;

pub struct TuiLayout {
    /// The entire TUI body.
    _body: Rc<[Rect]>,
    /// The console where build logs are displayed.
    console: Rc<[Rect]>,
    // The border that separates the two bars (info and status).
    border_sep: Rect,
    //. The status bar that displays build status, platform, versions, etc.
    status_bar: Rc<[Rect]>,
}

impl TuiLayout {
    pub fn new(frame_size: Rect) -> Self {
        // The full layout
        let body = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                // Console
                Constraint::Fill(1),
                // Border Separator
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
            .constraints([Constraint::Fill(1)])
            .split(body[0]);

        // Build the status bar.
        let status_bar = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Fill(1), Constraint::Fill(1)])
            .split(body[2]);

        // Specify borders
        let border_sep = body[1];

        Self {
            _body: body,
            console,
            border_sep,
            status_bar,
        }
    }

    /// Render all decorations.
    pub fn render_decor(&self, frame: &mut Frame) {
        frame.render_widget(
            Block::new()
                .borders(Borders::TOP)
                .border_style(Style::new().gray()),
            self.border_sep,
        );
    }

    /// Render the user's text selection
    pub fn render_selection(
        &self,
        frame: &mut Frame,
        drag_start: Option<(u16, u16)>,
        drag_end: Option<(u16, u16)>,
    ) -> Option<String> {
        let console = self.console[0];

        let start = drag_start?;
        let end = drag_end?;
        let buffer = frame.buffer_mut();

        let start_index = buffer.index_of(start.0, start.1);
        let end_index = buffer.index_of(end.0, end.1);
        let console_y_end = console.as_size().height - 1;

        let mut selected_lines = Vec::new();
        let direction_forward = start_index < end_index;

        // The drag was started out of console area.
        if start.1 > console_y_end {
            return None;
        }

        let mut i = start_index;
        loop {
            let (x, y) = buffer.pos_of(i);

            // Skip any cells outside of console area.
            if y > console_y_end {
                match direction_forward {
                    true => i += 1,
                    false => i -= 1,
                }
                if i == end_index {
                    break;
                }
                continue;
            }

            let cell = buffer.get_mut(x, y);
            cell.set_bg(Color::DarkGray);

            // Add each symbol to it's correct line.
            let symbol = cell.symbol();
            let line_index = match direction_forward {
                true => y - start.1,
                false => start.1 - y,
            } as usize;

            if let Some(line) = selected_lines.get_mut(line_index) {
                *line += symbol;
            } else {
                let line = String::from(symbol);
                selected_lines.push(line);
            }

            if i == end_index {
                break;
            }

            // Determine which direction we need to iterate through in the buffer.
            match direction_forward {
                true => i += 1,
                false => i -= 1,
            }
        }

        let mut final_text = String::new();

        // Go through each line, parse it, and append it to the final text.
        for line in selected_lines {
            let mut line = line;
            if !direction_forward {
                line = line.chars().rev().collect::<String>();
            }

            // Trim scroll bar and whitespace. 
            line = line.replace("▐", "").trim_end().to_string();
            if !line.is_empty() {
                writeln!(final_text, "{line}").unwrap();
            }
        }

        // Remove the last unescessary newline.
        final_text = final_text.trim_end_matches('\n').to_string();

        Some(final_text)
    }

    /// Render the console and it's logs.
    pub fn render_console(
        &self,
        frame: &mut Frame,
        scroll: ScrollPosition,
        messages: &[Message],
    ) -> NumLinesWrapping {
        // TODO: Fancy filtering support "show me only app logs from web"
        let console = self.console[0];
        let mut out_text = Text::default();

        // Filter logs for current tab.
        // Display in order they were created.
        let msgs = messages.iter();

        for msg in msgs {
            for line in msg.content.lines() {
                let text = line.into_text().unwrap_or_default();
                for line in text.lines {
                    // Don't add any formatting for cargo messages.
                    let out_line = if msg.source != MessageSource::Cargo {
                        let source = format!("[{}]", msg.source);
                        let source_span = Span::from(source);
                        let source_span = match msg.source {
                            MessageSource::App(_) => source_span.light_cyan(),
                            MessageSource::Dev => source_span.dark_gray(),
                            MessageSource::Build => source_span.light_yellow(),
                            MessageSource::Unknown => source_span.black(),
                            MessageSource::Cargo => {
                                unimplemented!("this shouldn't be reached")
                            }
                        };

                        let level = format!(" {}: ", msg.level);
                        let level_span = Span::from(level);
                        let level_span = match msg.level {
                            Level::TRACE => level_span.black(),
                            Level::DEBUG => level_span.light_magenta(),
                            Level::INFO => level_span.light_blue(),
                            Level::WARN => level_span.light_yellow(),
                            Level::ERROR => level_span.light_red(),
                        };

                        let mut out_line = vec![source_span, level_span];
                        for span in line.spans {
                            out_line.push(span);
                        }
                        out_line
                    } else {
                        line.spans
                    };

                    out_text.push_line(Line::from(out_line));
                }
            }
        }

        let paragraph = Paragraph::new(out_text)
            .left_aligned()
            .wrap(Wrap { trim: false });

        let console_height = self.get_console_height();
        let num_lines_wrapping = NumLinesWrapping(paragraph.line_count(console.width) as u16);

        // Render scrollbar
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

    /// Render the status bar.
    pub fn render_status_bar(
        &self,
        frame: &mut Frame,
        is_cli_release: bool,
        platform: Platform,
        build_progress: &BuildProgress,
        more_modal_open: bool,
    ) {
        // left aligned text
        let mut spans = vec![
            Span::from(if is_cli_release { "dx" } else { "dx-dev" }).green(),
            Span::from(" ").green(),
            Span::from("serve").green(),
            Span::from(" | ").white(),
            Span::from(platform.to_string()).green(),
            Span::from(" | ").white(),
        ];

        // If there is build progress, render the current status.
        let is_build_progress = !build_progress.current_builds.is_empty();
        if is_build_progress {
            // If the build failed, show a failed status.
            // Otherwise, render current status.
            let build_failed = build_progress
                .current_builds
                .values()
                .any(|b| b.failed.is_some());

            if build_failed {
                spans.push(Span::from("build failed ❌").red());
            } else {
                spans.push(Span::from("status: ").green());
                let build = build_progress
                    .current_builds
                    .values()
                    .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                    .unwrap();
                spans.extend_from_slice(&build.spans(Rect::new(0, 0, build.max_layout_size(), 1)));
            }
        }

        // right aligned text
        let more_span = Span::from("[/] more");
        let more_span = match more_modal_open {
            true => more_span.light_yellow(),
            false => more_span.gray(),
        };

        // Right-aligned text
        let right_line = Line::from(vec![
            more_span,
            Span::from(" | ").gray(),
            Span::from("[r] rebuild").gray(),
            Span::from(" | ").gray(),
            Span::from("[c] clear").gray(),
            Span::from(" | ").gray(),
            Span::from("[o] open").gray(),
        ]);

        frame.render_widget(
            Paragraph::new(Line::from(spans)).left_aligned(),
            self.status_bar[0],
        );

        // Render the info
        frame.render_widget(
            Paragraph::new(right_line).right_aligned(),
            self.status_bar[1],
        );
    }

    /// Renders the "more" modal to show extra info/keybinds accessible via the more keybind.
    pub fn render_more_modal(&self, frame: &mut Frame) {
        let area = self.console[0];
        let modal = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Fill(1)])
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
