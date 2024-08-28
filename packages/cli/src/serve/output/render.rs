//! This module contains functions to render different elements on the TUI frame.
//!
//! The current TUI layout is:
//! ------------
//! -- CONSOLE--
//! ------------
//! ---BORDER---
//! {OPT DRAWER}
//! {OPT BORDER}
//! -STATUS BAR-

use super::{BuildProgress, Message, MessageSource};
use ansi_to_tui::IntoText as _;
use dioxus_cli_config::Platform;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{
        Block, Borders, Clear, List, ListState, Paragraph, Scrollbar, ScrollbarOrientation,
        ScrollbarState, Widget, Wrap,
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

        let mut console_constraints = vec![Constraint::Fill(1)];
        if filter_open {
            console_constraints.push(Constraint::Length(1));
            console_constraints.push(Constraint::Length(25));
        }

        // Build the console, where logs go.
        let console = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(console_constraints)
            .split(body[0]);

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
            .split(body[2]);

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

    /// Render the user's text selection and compile it into a list of lines.
    pub fn render_selection(
        &self,
        frame: &mut Frame,
        drag_start: Option<(u16, u16)>,
        drag_end: Option<(u16, u16)>,
        selected_lines: &mut Vec<String>,
    ) {
        let console = self.console[0];

        let Some(start) = drag_start else {
            return;
        };
        let Some(end) = drag_end else {
            return;
        };

        let buffer = frame.buffer_mut();

        let start_index = buffer.index_of(start.0, start.1);
        let end_index = buffer.index_of(end.0, end.1);

        let console_size = console.as_size();
        let console_x_end = console_size.width;
        let console_y_end = console_size.height;

        let mut new_selected_lines = Vec::new();
        let direction_forward = start_index < end_index;

        // The drag was started out of console area.
        if start.0 > console_x_end || start.1 > console_y_end {
            return;
        }

        let mut i = start_index;
        loop {
            if i == end_index {
                break;
            }

            let (x, y) = buffer.pos_of(i);

            // Skip any cells outside of console area.
            // This looping logic is a bit duplicated.
            if y >= console_y_end || x >= console_x_end {
                match direction_forward {
                    true => i = i.saturating_add(1),
                    false => i = i.saturating_sub(1),
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

            // Add the symbol to it's correct line, creating it if null.
            if let Some(line) = new_selected_lines.get_mut(line_index) {
                *line += symbol;
            } else {
                let line = String::from(symbol);
                new_selected_lines.push(line);
            }

            // Determine which direction we need to iterate through in the buffer.
            match direction_forward {
                true => i = i.saturating_add(1),
                false => i = i.saturating_sub(1),
            }
        }

        if !direction_forward {
            new_selected_lines.reverse();
        }

        // Replace current selected lines with new ones.
        selected_lines.clear();
        for line in new_selected_lines.iter_mut() {
            // Reverse lines if needed.
            if !direction_forward {
                *line = line.chars().rev().collect::<String>();
            }

            selected_lines.push(line.clone());
        }
    }

    /// Render the console and it's logs, returning the number of lines required to render the entire log output.
    pub fn render_console(
        &self,
        frame: &mut Frame,
        scroll_position: u16,
        messages: &[Message],
    ) -> u16 {
        // TODO: Fancy filtering support "show me only app logs from web"
        let console = self.console[0];
        let mut out_text = Text::default();

        // Filter logs for current tab.
        // Display in order they were created.
        let msgs = messages.iter();

        // Find the largest prefix sizes before assembly the messages.
        let mut source_len = 0;
        let mut level_len = 0;
        for msg in msgs {
            let source = format!("[{}]", msg.source);
            let len = source.len();
            if len > source_len {
                source_len = len;
            }

            let level = format!("{}:", msg.level);
            let len = level.len();
            if len > level_len {
                level_len = len;
            }
        }

        let (console_width, console_height) = self.get_console_size();
        let msgs = messages.iter();

        // Assemble the messages
        for msg in msgs {
            let mut sub_line_padding = 0;
            let mut first_line = true;

            let text = msg.content.into_text().unwrap_or_default();

            for line in text.lines {
                // Don't add any formatting for cargo messages.
                let out_line = if msg.source != MessageSource::Cargo {
                    if first_line {
                        first_line = false;

                        // Build source tag: `[dev]`
                        // We subtract 2 here to account for the `[]`
                        let padding =
                            build_msg_padding(source_len - msg.source.to_string().len() - 2);
                        let source = format!("{}[{}]", padding, msg.source);
                        sub_line_padding += source.len();

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

                        // Build level tag: `INFO:``
                        // We don't subtract 1 here for `:` because we still want at least 1 padding.
                        let padding = build_msg_padding(level_len - msg.level.to_string().len());
                        let level = format!("{}{}: ", padding, msg.level);
                        sub_line_padding += level.len();

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

                out_text.push_line(Line::from(out_line));
            }
        }

        // Add an extra line since scroll can't hit last line for some reason.
        out_text.push_line(Line::from(""));

        let paragraph = Paragraph::new(out_text)
            .left_aligned()
            .wrap(Wrap { trim: false });

        let num_lines_wrapping = paragraph.line_count(console_width) as u16;

        // Render scrollbar
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None)
            .track_symbol(None)
            .thumb_symbol("▐");

        let mut scrollbar_state =
            ScrollbarState::new(num_lines_wrapping.saturating_sub(console_height) as usize)
                .position(scroll_position as usize);

        let paragraph = paragraph.scroll((scroll_position, 0));
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
        filter_menu_open: bool,
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

        let filter_span = Span::from("[f] filter");
        let filter_span = match filter_menu_open {
            true => filter_span.light_yellow(),
            false => filter_span.gray(),
        };

        // Right-aligned text
        let right_line = Line::from(vec![
            more_span,
            Span::from(" | ").gray(),
            Span::from("[r] rebuild").gray(),
            Span::from(" | ").gray(),
            filter_span,
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
                Constraint::Length(5),
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
            Line::from("[↑] Up").white(),
            Line::from("[↓] Down").white(),
            Line::from("[→] Toggle").white(),
            Line::from("[enter] Type / Submit").white(),
        ];
        let text = Text::from(lines);
        frame.render_widget(text, keybinds);
    }

    // /// Generate the paragraph for the filter drawer.
    // pub fn get_filter_drawer_text<'a>(
    //     enabled_filters: &[MessageFilter],
    //     selected_filter_index: usize,
    //     search_input: String,
    // ) -> Paragraph<'a> {
    //     let mut spans = vec![Span::from("Filters: ").light_blue()];

    //     for (i, filter) in AVAILABLE_FILTERS.iter().enumerate() {
    //         let mut span = Span::from(filter.to_string()).dark_gray();
    //         if enabled_filters.contains(filter) {
    //             span = span.light_yellow();
    //         }

    //         // Add arrow prefix if currently focused
    //         if selected_filter_index == i {
    //             let prefix = Span::from("» ").gray();
    //             spans.push(prefix);
    //         }

    //         spans.push(span);

    //         let postfix = Span::from(", ").dark_gray();
    //         spans.push(postfix);
    //     }

    //     let mut other_spans = vec![
    //         Span::from("| ").gray(),
    //         Span::from("[<] ").dark_gray(),
    //         Span::from("left ").gray(),
    //         Span::from("[>] ").dark_gray(),
    //         Span::from("right ").gray(),
    //         Span::from("[enter] ").dark_gray(),
    //         Span::from("toggle filter ").gray(),
    //         Span::from("| Search: ").gray(),
    //         Span::from(search_input).dark_gray(),
    //     ];

    //     spans.append(&mut other_spans);
    //     let line = Line::from(spans);

    //     Paragraph::new(line)
    //         .alignment(Alignment::Left)
    //         .wrap(Wrap { trim: false })
    // }

    /// Returns the height of the console TUI area in number of lines.
    pub fn get_console_size(&self) -> (u16, u16) {
        (self.console[0].width, self.console[0].height)
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
