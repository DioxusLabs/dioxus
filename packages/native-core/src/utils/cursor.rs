use std::cmp::Ordering;

use dioxus_html::input_data::keyboard_types::{Code, Key, Modifiers};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pos {
    pub col: usize,
    pub row: usize,
}

impl Pos {
    pub fn new(col: usize, row: usize) -> Self {
        Self { row, col }
    }

    pub fn up(&mut self, rope: &str) {
        self.move_row(-1, rope);
    }

    pub fn down(&mut self, rope: &str) {
        self.move_row(1, rope);
    }

    pub fn right(&mut self, rope: &str) {
        self.move_col(1, rope);
    }

    pub fn left(&mut self, rope: &str) {
        self.move_col(-1, rope);
    }

    pub fn move_row(&mut self, change: i32, rope: &str) {
        let new = self.row as i32 + change;
        if new >= 0 && new < rope.lines().count() as i32 {
            self.row = new as usize;
        }
    }

    pub fn move_col(&mut self, change: i32, rope: &str) {
        self.realize_col(rope);
        let idx = self.idx(rope) as i32;
        if idx + change >= 0 && idx + change <= rope.len() as i32 {
            let len_line = self.len_line(rope) as i32;
            let new_col = self.col as i32 + change;
            let diff = new_col - len_line;
            if diff > 0 {
                self.down(rope);
                self.col = 0;
                self.move_col(diff - 1, rope);
            } else if new_col < 0 {
                self.up(rope);
                self.col = self.len_line(rope);
                self.move_col(new_col + 1, rope);
            } else {
                self.col = new_col as usize;
            }
        }
    }

    pub fn col(&self, rope: &str) -> usize {
        self.col.min(self.len_line(rope))
    }

    pub fn row(&self) -> usize {
        self.row
    }

    fn len_line(&self, rope: &str) -> usize {
        let line = rope.lines().nth(self.row).unwrap_or_default();
        let len = line.len();
        if len > 0 && line.chars().nth(len - 1) == Some('\n') {
            len - 1
        } else {
            len
        }
    }

    pub fn idx(&self, rope: &str) -> usize {
        rope.lines().take(self.row).map(|l| l.len()).sum::<usize>() + self.col(rope)
    }

    // the column can be more than the line length, cap it
    pub fn realize_col(&mut self, rope: &str) {
        self.col = self.col(rope);
    }
}

impl Ord for Pos {
    fn cmp(&self, other: &Self) -> Ordering {
        self.row.cmp(&other.row).then(self.col.cmp(&other.col))
    }
}

impl PartialOrd for Pos {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cursor {
    pub start: Pos,
    pub end: Option<Pos>,
}

impl Cursor {
    pub fn from_start(pos: Pos) -> Self {
        Self {
            start: pos,
            end: None,
        }
    }

    pub fn new(start: Pos, end: Pos) -> Self {
        Self {
            start,
            end: Some(end),
        }
    }

    fn move_cursor(&mut self, f: impl FnOnce(&mut Pos), shift: bool) {
        if shift {
            self.with_end(f);
        } else {
            f(&mut self.start);
            self.end = None;
        }
    }

    fn delete_selection(&mut self, text: &mut String) -> [i32; 2] {
        let first = self.first();
        let last = self.last();
        let dr = first.row as i32 - last.row as i32;
        let dc = if dr != 0 {
            -(last.col as i32)
        } else {
            first.col as i32 - last.col as i32
        };
        text.replace_range(first.idx(text)..last.idx(text), "");
        if let Some(end) = self.end.take() {
            if self.start > end {
                self.start = end;
            }
        }
        [dc, dr]
    }

    pub fn handle_input(
        &mut self,
        data: &dioxus_html::KeyboardData,
        text: &mut String,
        max_width: usize,
    ) {
        use Code::*;
        match data.code() {
            ArrowUp => {
                self.move_cursor(|c| c.up(text), data.modifiers().contains(Modifiers::SHIFT));
            }
            ArrowDown => {
                self.move_cursor(
                    |c| c.down(text),
                    data.modifiers().contains(Modifiers::SHIFT),
                );
            }
            ArrowRight => {
                if data.modifiers().contains(Modifiers::CONTROL) {
                    self.move_cursor(
                        |c| {
                            let mut change = 1;
                            let idx = c.idx(text);
                            let length = text.len();
                            while idx + change < length {
                                let chr = text.chars().nth(idx + change).unwrap();
                                if chr.is_whitespace() {
                                    break;
                                }
                                change += 1;
                            }
                            c.move_col(change as i32, text);
                        },
                        data.modifiers().contains(Modifiers::SHIFT),
                    );
                } else {
                    self.move_cursor(
                        |c| c.right(text),
                        data.modifiers().contains(Modifiers::SHIFT),
                    );
                }
            }
            ArrowLeft => {
                if data.modifiers().contains(Modifiers::CONTROL) {
                    self.move_cursor(
                        |c| {
                            let mut change = -1;
                            let idx = c.idx(text) as i32;
                            while idx + change > 0 {
                                let chr = text.chars().nth((idx + change) as usize).unwrap();
                                if chr == ' ' {
                                    break;
                                }
                                change -= 1;
                            }
                            c.move_col(change, text);
                        },
                        data.modifiers().contains(Modifiers::SHIFT),
                    );
                } else {
                    self.move_cursor(
                        |c| c.left(text),
                        data.modifiers().contains(Modifiers::SHIFT),
                    );
                }
            }
            End => {
                self.move_cursor(
                    |c| c.col = c.len_line(text),
                    data.modifiers().contains(Modifiers::SHIFT),
                );
            }
            Home => {
                self.move_cursor(|c| c.col = 0, data.modifiers().contains(Modifiers::SHIFT));
            }
            Backspace => {
                self.start.realize_col(text);
                let mut start_idx = self.start.idx(text);
                if self.end.is_some() {
                    self.delete_selection(text);
                } else if start_idx > 0 {
                    self.start.left(text);
                    text.replace_range(start_idx - 1..start_idx, "");
                    if data.modifiers().contains(Modifiers::CONTROL) {
                        start_idx = self.start.idx(text);
                        while start_idx > 0
                            && text
                                .chars()
                                .nth(start_idx - 1)
                                .filter(|c| *c != ' ')
                                .is_some()
                        {
                            self.start.left(text);
                            text.replace_range(start_idx - 1..start_idx, "");
                            start_idx = self.start.idx(text);
                        }
                    }
                }
            }
            Enter => {
                if text.len() + 1 - self.selection_len(text) <= max_width {
                    text.insert(self.start.idx(text), '\n');
                    self.start.col = 0;
                    self.start.down(text);
                }
            }
            Tab => {
                if text.len() + 1 - self.selection_len(text) <= max_width {
                    self.start.realize_col(text);
                    self.delete_selection(text);
                    text.insert(self.start.idx(text), '\t');
                    self.start.right(text);
                }
            }
            _ => {
                self.start.realize_col(text);
                if let Key::Character(character) = data.key() {
                    if text.len() + 1 - self.selection_len(text) <= max_width {
                        self.delete_selection(text);
                        let character = character.chars().next().unwrap();
                        text.insert(self.start.idx(text), character);
                        self.start.right(text);
                    }
                }
            }
        }
    }

    pub fn with_end(&mut self, f: impl FnOnce(&mut Pos)) {
        let mut new = self.end.take().unwrap_or_else(|| self.start.clone());
        f(&mut new);
        self.end.replace(new);
    }

    pub fn first(&self) -> &Pos {
        if let Some(e) = &self.end {
            e.min(&self.start)
        } else {
            &self.start
        }
    }

    pub fn last(&self) -> &Pos {
        if let Some(e) = &self.end {
            e.max(&self.start)
        } else {
            &self.start
        }
    }

    pub fn selection_len(&self, text: &str) -> usize {
        self.last().idx(text) - self.first().idx(text)
    }
}

impl Default for Cursor {
    fn default() -> Self {
        Self {
            start: Pos::new(0, 0),
            end: None,
        }
    }
}

#[test]
fn pos_direction_movement() {
    let mut pos = Pos::new(100, 0);
    let text = "hello world\nhi";

    assert_eq!(pos.col(text), text.lines().next().unwrap_or_default().len());
    pos.down(text);
    assert_eq!(pos.col(text), text.lines().nth(1).unwrap_or_default().len());
    pos.up(text);
    assert_eq!(pos.col(text), text.lines().next().unwrap_or_default().len());
    pos.left(text);
    assert_eq!(
        pos.col(text),
        text.lines().next().unwrap_or_default().len() - 1
    );
    pos.right(text);
    assert_eq!(pos.col(text), text.lines().next().unwrap_or_default().len());
}

#[test]
fn pos_col_movement() {
    let mut pos = Pos::new(100, 0);
    let text = "hello world\nhi";

    // move inside a row
    pos.move_col(-5, text);
    assert_eq!(
        pos.col(text),
        text.lines().next().unwrap_or_default().len() - 5
    );
    pos.move_col(5, text);
    assert_eq!(pos.col(text), text.lines().next().unwrap_or_default().len());

    // move between rows
    pos.move_col(3, text);
    assert_eq!(pos.col(text), 2);
    pos.move_col(-3, text);
    assert_eq!(pos.col(text), text.lines().next().unwrap_or_default().len());

    // don't panic if moving out of range
    pos.move_col(-100, text);
    pos.move_col(1000, text);
}

#[test]
fn cursor_row_movement() {
    let mut pos = Pos::new(100, 0);
    let text = "hello world\nhi";

    pos.move_row(1, text);
    assert_eq!(pos.row(), 1);
    pos.move_row(-1, text);
    assert_eq!(pos.row(), 0);

    // don't panic if moving out of range
    pos.move_row(-100, text);
    pos.move_row(1000, text);
}

#[test]
fn cursor_input() {
    let mut cursor = Cursor::from_start(Pos::new(0, 0));
    let mut text = "hello world\nhi".to_string();

    for _ in 0..5 {
        cursor.handle_input(
            &dioxus_html::KeyboardData::new(
                dioxus_html::input_data::keyboard_types::Key::ArrowRight,
                dioxus_html::input_data::keyboard_types::Code::ArrowRight,
                dioxus_html::input_data::keyboard_types::Location::Standard,
                false,
                Modifiers::empty(),
            ),
            &mut text,
            10,
        );
    }

    for _ in 0..5 {
        cursor.handle_input(
            &dioxus_html::KeyboardData::new(
                dioxus_html::input_data::keyboard_types::Key::Backspace,
                dioxus_html::input_data::keyboard_types::Code::Backspace,
                dioxus_html::input_data::keyboard_types::Location::Standard,
                false,
                Modifiers::empty(),
            ),
            &mut text,
            10,
        );
    }

    assert_eq!(text, " world\nhi");

    let goal_text = "hello world\nhi";
    let max_width = goal_text.len();
    cursor.handle_input(
        &dioxus_html::KeyboardData::new(
            dioxus_html::input_data::keyboard_types::Key::Character("h".to_string()),
            dioxus_html::input_data::keyboard_types::Code::KeyH,
            dioxus_html::input_data::keyboard_types::Location::Standard,
            false,
            Modifiers::empty(),
        ),
        &mut text,
        max_width,
    );

    cursor.handle_input(
        &dioxus_html::KeyboardData::new(
            dioxus_html::input_data::keyboard_types::Key::Character("e".to_string()),
            dioxus_html::input_data::keyboard_types::Code::KeyE,
            dioxus_html::input_data::keyboard_types::Location::Standard,
            false,
            Modifiers::empty(),
        ),
        &mut text,
        max_width,
    );

    cursor.handle_input(
        &dioxus_html::KeyboardData::new(
            dioxus_html::input_data::keyboard_types::Key::Character("l".to_string()),
            dioxus_html::input_data::keyboard_types::Code::KeyL,
            dioxus_html::input_data::keyboard_types::Location::Standard,
            false,
            Modifiers::empty(),
        ),
        &mut text,
        max_width,
    );

    cursor.handle_input(
        &dioxus_html::KeyboardData::new(
            dioxus_html::input_data::keyboard_types::Key::Character("l".to_string()),
            dioxus_html::input_data::keyboard_types::Code::KeyL,
            dioxus_html::input_data::keyboard_types::Location::Standard,
            false,
            Modifiers::empty(),
        ),
        &mut text,
        max_width,
    );

    cursor.handle_input(
        &dioxus_html::KeyboardData::new(
            dioxus_html::input_data::keyboard_types::Key::Character("o".to_string()),
            dioxus_html::input_data::keyboard_types::Code::KeyO,
            dioxus_html::input_data::keyboard_types::Location::Standard,
            false,
            Modifiers::empty(),
        ),
        &mut text,
        max_width,
    );

    // these should be ignored
    for _ in 0..10 {
        cursor.handle_input(
            &dioxus_html::KeyboardData::new(
                dioxus_html::input_data::keyboard_types::Key::Character("o".to_string()),
                dioxus_html::input_data::keyboard_types::Code::KeyO,
                dioxus_html::input_data::keyboard_types::Location::Standard,
                false,
                Modifiers::empty(),
            ),
            &mut text,
            max_width,
        );
    }

    assert_eq!(text.to_string(), goal_text);
}
