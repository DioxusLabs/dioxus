//! This module contains functions to render different elements on the TUI frame.
// TODO: Cleanup console filtering / message building logic

use super::{BuildProgress, Message, TraceSrc};
use ansi_to_tui::IntoText as _;
use dioxus_cli_config::Platform;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListState, Paragraph, Widget, Wrap},
    Frame,
};
use regex::Regex;
use std::fmt::Write as _;
use std::rc::Rc;
use tracing::Level;

pub struct TuiLayout {
    /// The entire TUI body.
    _body: Rc<[Rect]>,
    /// The console where build logs are displayed.
    console: Rc<[Rect]>,
    // The filter drawer if the drawer is open.
    filter_drawer: Option<Rc<[Rect]>>,

    // The border that separates the console and info bars.
    border_sep: Rect,

    //. The status bar that displays build status, platform, versions, etc.
    status_bar: Rc<[Rect]>,

    // Misc
    filter_list_state: ListState,
}

impl TuiLayout {
    pub fn new(frame_size: Rect, filter_open: bool) -> Self {
        // The full layout
        let body = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                // Footer Status
                Constraint::Length(1),
                // Border Separator
                Constraint::Length(1),
                // Console
                Constraint::Fill(1),
                // Padding
                Constraint::Length(1),
            ])
            .split(frame_size);

        let mut console_constraints = vec![Constraint::Fill(1)];
        if filter_open {
            console_constraints.push(Constraint::Length(1));
            console_constraints.push(Constraint::Length(25));
        }

        // Build the console, where logs go.
        let console = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(console_constraints)
            .split(body[2]);

        let filter_drawer = match filter_open {
            false => None,
            true => Some(
                Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Length(1),
                        Constraint::Fill(1),
                        Constraint::Length(1),
                    ])
                    .split(console[2]),
            ),
        };

        // Build the status bar.
        let status_bar = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Fill(1), Constraint::Fill(1)])
            .split(body[0]);

        // Specify borders
        let border_sep_top = body[1];

        Self {
            _body: body,
            console,
            filter_drawer,
            border_sep: border_sep_top,
            status_bar,
            filter_list_state: ListState::default(),
        }
    }

    /// Render all decorations.
    pub fn render_decor(&self, frame: &mut Frame, filter_open: bool) {
        frame.render_widget(
            Block::new()
                .borders(Borders::TOP)
                .border_style(Style::new().white()),
            self.border_sep,
        );

        if filter_open {
            frame.render_widget(
                Block::new()
                    .borders(Borders::LEFT)
                    .border_style(Style::new().white()),
                self.console[1],
            );
        }
    }

    /// Render the console and it's logs, returning the number of lines required to render the entire log output.
    pub fn render_console(
        &self,
        frame: &mut Frame,
        scroll_position: u16,
        messages: &[Message],
        enabled_filters: &[String],
    ) -> u16 {
        let console = self.console[0];
        let mut out_text = Text::default();

        let level_len = "BUILD: ".len();
        let (console_width, _console_height) = self.get_console_size();
        let msgs = messages.iter();

        // Assemble the messages
        for msg in msgs {
            let mut sub_line_padding = 0;

            let text = msg.content.trim_end().into_text().unwrap_or_default();

            for (idx, line) in text.lines.into_iter().enumerate() {
                // Don't add any formatting for cargo messages.
                let out_line = if msg.source != TraceSrc::Cargo {
                    if idx == 0 {
                        match msg.source {
                            TraceSrc::Dev => {
                                let mut spans =
                                    vec![Span::from(format!("  DEV: ",)).light_magenta()];

                                for span in line.spans {
                                    spans.push(span);
                                }
                                spans
                            }
                            TraceSrc::Build => {
                                let mut spans = vec![Span::from(format!("BUILD: ",)).light_blue()];

                                for span in line.spans {
                                    spans.push(span);
                                }
                                spans
                            }
                            _ => {
                                // Build level tag: `INFO:``
                                // We don't subtract 1 here for `:` because we still want at least 1 padding.
                                let padding =
                                    build_msg_padding(level_len - msg.level.to_string().len() - 2);
                                let level = format!("{padding}{}: ", msg.level);
                                sub_line_padding += level.len();

                                let level_span = Span::from(level);
                                let level_span = match msg.level {
                                    Level::TRACE => level_span.black(),
                                    Level::DEBUG => level_span.light_magenta(),
                                    Level::INFO => level_span.light_green(),
                                    Level::WARN => level_span.light_yellow(),
                                    Level::ERROR => level_span.light_red(),
                                };

                                let mut out_line = vec![level_span];
                                for span in line.spans {
                                    out_line.push(span);
                                }

                                out_line
                            }
                        }
                    } else {
                        // Not the first line. Append the padding and merge into list.
                        let padding = build_msg_padding(sub_line_padding);

                        let mut out_line = vec![Span::from(padding)];
                        for span in line.spans {
                            out_line.push(span);
                        }
                        out_line
                    }
                } else {
                    line.spans
                };

                if msg.source != TraceSrc::Cargo {
                    out_text.push_line(Line::from(out_line));
                }

                // out_text.push_line(Line::from(out_line));
            }
        }

        // Only show messages for filters that are enabled.
        let mut included_line_ids = Vec::new();

        for filter in enabled_filters {
            let re = Regex::new(filter);
            for (index, line) in out_text.lines.iter().enumerate() {
                let line_str = line.to_string();
                match re {
                    Ok(ref re) => {
                        // sort by provided regex
                        if re.is_match(&line_str) {
                            included_line_ids.push(index);
                        }
                    }
                    Err(_) => {
                        // default to basic string storing
                        if line_str.contains(filter) {
                            included_line_ids.push(index);
                        }
                    }
                }
            }
        }

        included_line_ids.sort_unstable();
        included_line_ids.dedup();

        let out_lines = out_text.lines;
        let mut out_text = Text::default();

        if enabled_filters.is_empty() {
            for line in out_lines {
                out_text.push_line(line.clone());
            }
        } else {
            for id in included_line_ids {
                if let Some(line) = out_lines.get(id) {
                    out_text.push_line(line.clone());
                }
            }
        }

        let paragraph = Paragraph::new(out_text)
            .left_aligned()
            .wrap(Wrap { trim: false });

        let num_lines_wrapping = paragraph.line_count(console_width) as u16;

        let paragraph = paragraph.scroll((scroll_position, 0));
        paragraph.render(console, frame.buffer_mut());

        num_lines_wrapping
    }

    /// Render the status bar.
    pub fn render_status_bar(
        &self,
        frame: &mut Frame,
        _is_cli_release: bool,
        _platform: Platform,
        build_progress: &BuildProgress,
        more_modal_open: bool,
        filter_menu_open: bool,
        dx_version: &str,
    ) {
        // left aligned text
        let mut spans = vec![
            Span::from("🧬 dx").white(),
            Span::from(" ").white(),
            Span::from(format!("{}", dx_version)).white(),
            Span::from(" | ").dark_gray(),
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
                spans.push(Span::from("Build failed ❌").red());
            } else {
                // spans.push(Span::from("status: ").gray());
                let build = build_progress
                    .current_builds
                    .values()
                    .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                    .unwrap();
                spans.extend_from_slice(&build.make_spans(Rect::new(
                    0,
                    0,
                    build.max_layout_size(),
                    1,
                )));
            }
        }

        // right aligned text
        let more_span = Span::from("[/] more");
        let more_span = match more_modal_open {
            true => more_span.light_yellow(),
            false => more_span.gray(),
        };

        let filter_span = Span::from("[f] filter");
        let filter_span = match filter_menu_open {
            true => filter_span.light_yellow(),
            false => filter_span.gray(),
        };

        // Right-aligned text
        let right_line = Line::from(vec![
            Span::from("[o] open").gray(),
            Span::from(" | ").gray(),
            Span::from("[r] rebuild").gray(),
            Span::from(" | ").gray(),
            filter_span,
            Span::from(" | ").dark_gray(),
            more_span,
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
            .constraints([Constraint::Fill(1), Constraint::Length(5)])
            .split(area)[1];

        frame.render_widget(Clear, modal);
        frame.render_widget(Block::default().borders(Borders::ALL), modal);

        // Render under construction message
        let msg = Paragraph::new("Under construction, please check back at a later date!")
            .alignment(Alignment::Center);
        frame.render_widget(msg, modal);
    }

    /// Render the filter drawer menu.
    pub fn render_filter_menu(
        &mut self,
        frame: &mut Frame,
        filters: &[(String, bool)],
        selected_filter_index: usize,
        search_mode: bool,
        search_input: Option<&String>,
    ) {
        let Some(ref filter_drawer) = self.filter_drawer else {
            return;
        };

        // Vertical layout
        let container = Layout::default()
            .constraints([
                Constraint::Length(4),
                Constraint::Fill(1),
                Constraint::Length(7),
            ])
            .direction(Direction::Vertical)
            .split(filter_drawer[1]);

        // Render the search section.
        let top_area = Layout::default()
            .constraints([
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
            .direction(Direction::Vertical)
            .split(container[0]);

        let search_title = Line::from("Search").gray();
        let search_input_block = Block::new().bg(Color::White);

        let search_text = match search_input {
            Some(s) => s,
            None => {
                if search_mode {
                    "..."
                } else {
                    "[enter] to type..."
                }
            }
        };

        let search_input = Paragraph::new(Line::from(search_text))
            .fg(Color::Black)
            .block(search_input_block);

        frame.render_widget(search_title, top_area[1]);
        frame.render_widget(search_input, top_area[2]);

        // Render the filters
        let list_area = container[1];
        let mut list_items = Vec::new();

        for (filter, enabled) in filters {
            let filter = Span::from(filter);
            let filter = match enabled {
                true => filter.light_yellow(),
                false => filter.dark_gray(),
            };
            list_items.push(filter);
        }
        list_items.reverse();

        let list = List::new(list_items).highlight_symbol("» ");
        self.filter_list_state.select(Some(selected_filter_index));
        frame.render_stateful_widget(list, list_area, &mut self.filter_list_state);

        // Render the keybind list at the bottom.
        let keybinds = container[2];
        let lines = vec![
            Line::from(""),
            Line::from("[↑] Up").white(),
            Line::from("[↓] Down").white(),
            Line::from("[←] Remove").white(),
            Line::from("[→] Toggle").white(),
            Line::from("[enter] Type / Submit").white(),
        ];
        let text = Text::from(lines);
        frame.render_widget(text, keybinds);
    }

    /// Returns the height of the console TUI area in number of lines.
    pub fn get_console_size(&self) -> (u16, u16) {
        (self.console[0].width, self.console[0].height)
    }

    /// Render the current scroll position at the top right corner of the frame
    pub(crate) fn render_current_scroll(
        &self,
        scroll_position: u16,
        lines: u16,
        console_height: u16,
        frame: &mut Frame<'_>,
    ) {
        let area = self.console[0];
        let mut row = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1)])
            .split(area)[0];

        row.y -= 1;

        let max_scroll = lines.saturating_sub(console_height);
        if max_scroll == 0 {
            return;
        }

        let remaining_ines = max_scroll.saturating_sub(scroll_position);

        // row.x -= (3 - remaining_ines.to_string().len()) as u16;

        if remaining_ines != 0 {
            let text = vec![Span::from(format!(" {remaining_ines}⬇ ").dark_gray())];

            let msg = Paragraph::new(Line::from(text))
                .alignment(Alignment::Right)
                .block(Block::default());

            frame.render_widget(msg, row);
        }
    }
}

/// Generate a string with a specified number of spaces.
fn build_msg_padding(padding_len: usize) -> String {
    let mut padding = String::new();
    for _ in 0..padding_len {
        _ = write!(padding, " ");
    }
    padding
}
