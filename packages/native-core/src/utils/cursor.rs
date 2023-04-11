//! A cursor implementation that can be used to navigate and edit text.

use std::{cmp::Ordering, ops::Range};

use keyboard_types::{Code, Key, Modifiers};

/// This contains the information about the text that is used by the cursor to handle navigation.
pub trait Text {
    /// Returns the line at the given index.
    fn line(&self, number: usize) -> Option<&Self>;
    /// Returns the length of the text in characters.
    fn length(&self) -> usize;
    /// Returns the number of lines in the text.
    fn line_count(&self) -> usize;
    /// Returns the character at the given character index.
    fn character(&self, idx: usize) -> Option<char>;
    /// Returns the length of the text before the given line in characters.
    fn len_before_line(&self, line: usize) -> usize;
}

impl Text for str {
    fn line(&self, number: usize) -> Option<&str> {
        self.lines().nth(number)
    }

    fn length(&self) -> usize {
        self.chars().count()
    }

    fn line_count(&self) -> usize {
        self.lines().count()
    }

    fn character(&self, idx: usize) -> Option<char> {
        self.chars().nth(idx)
    }

    fn len_before_line(&self, line: usize) -> usize {
        self.lines()
            .take(line)
            .map(|l| l.chars().count())
            .sum::<usize>()
    }
}

/// This contains the information about the text that is used by the cursor to handle editing text.
pub trait TextEditable<T: Text + ?Sized>: AsRef<T> {
    /// Inserts a character at the given character index.
    fn insert_character(&mut self, idx: usize, text: char);
    /// Deletes the given character range.
    fn delete_range(&mut self, range: Range<usize>);
}

impl TextEditable<str> for String {
    fn insert_character(&mut self, idx: usize, text: char) {
        self.insert(idx, text);
    }

    fn delete_range(&mut self, range: Range<usize>) {
        self.replace_range(range, "");
    }
}

/// A cursor position
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pos {
    /// The virtual column of the cursor. This can be more than the line length. To get the realized column, use [`Pos::col()`].
    pub col: usize,
    /// The row of the cursor.
    pub row: usize,
}

impl Pos {
    /// Creates a new cursor position.
    pub fn new(col: usize, row: usize) -> Self {
        Self { row, col }
    }

    /// Moves the position up by one line.
    pub fn up(&mut self, text: &(impl Text + ?Sized)) {
        self.move_row(-1, text);
    }

    /// Moves the position down by one line.
    pub fn down(&mut self, text: &(impl Text + ?Sized)) {
        self.move_row(1, text);
    }

    /// Moves the position right by one character.
    pub fn right(&mut self, text: &(impl Text + ?Sized)) {
        self.move_col(1, text);
    }

    /// Moves the position left by one character.
    pub fn left(&mut self, text: &(impl Text + ?Sized)) {
        self.move_col(-1, text);
    }

    /// Move the position's row by the given amount. (positive is down, negative is up)
    pub fn move_row(&mut self, change: i32, text: &(impl Text + ?Sized)) {
        let new = self.row as i32 + change;
        if new >= 0 && new < text.line_count() as i32 {
            self.row = new as usize;
        }
    }

    /// Move the position's column by the given amount. (positive is right, negative is left)
    pub fn move_col(&mut self, change: i32, text: &(impl Text + ?Sized)) {
        self.realize_col(text);
        let idx = self.idx(text) as i32;
        if idx + change >= 0 && idx + change <= text.length() as i32 {
            let len_line = self.len_line(text) as i32;
            let new_col = self.col as i32 + change;
            let diff = new_col - len_line;
            if diff > 0 {
                self.down(text);
                self.col = 0;
                self.move_col(diff - 1, text);
            } else if new_col < 0 {
                self.up(text);
                self.col = self.len_line(text);
                self.move_col(new_col + 1, text);
            } else {
                self.col = new_col as usize;
            }
        }
    }

    /// Get the realized column of the position. This is the column, but capped at the line length.
    pub fn col(&self, text: &(impl Text + ?Sized)) -> usize {
        self.col.min(self.len_line(text))
    }

    /// Get the row of the position.
    pub fn row(&self) -> usize {
        self.row
    }

    fn len_line(&self, text: &(impl Text + ?Sized)) -> usize {
        if let Some(line) = text.line(self.row) {
            let len = line.length();
            if len > 0 && line.character(len - 1) == Some('\n') {
                len - 1
            } else {
                len
            }
        } else {
            0
        }
    }

    /// Get the character index of the position.
    pub fn idx(&self, text: &(impl Text + ?Sized)) -> usize {
        text.len_before_line(self.row) + self.col(text)
    }

    /// If the column is more than the line length, cap it to the line length.
    pub fn realize_col(&mut self, text: &(impl Text + ?Sized)) {
        self.col = self.col(text);
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

/// A cursor is a selection of text. It has a start and end position of the selection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cursor {
    /// The start position of the selection. The start position is the origin of the selection, not necessarily the first position.
    pub start: Pos,
    /// The end position of the selection. If the end position is None, the cursor is a caret.
    pub end: Option<Pos>,
}

impl Cursor {
    /// Create a new cursor with the given start position.
    pub fn from_start(pos: Pos) -> Self {
        Self {
            start: pos,
            end: None,
        }
    }

    /// Create a new cursor with the given start and end position.
    pub fn new(start: Pos, end: Pos) -> Self {
        Self {
            start,
            end: Some(end),
        }
    }

    /// Move the cursor position. If shift is true, the end position will be moved instead of the start position.
    pub fn move_cursor(&mut self, f: impl FnOnce(&mut Pos), shift: bool) {
        if shift {
            self.with_end(f);
        } else {
            f(&mut self.start);
            self.end = None;
        }
    }

    /// Delete the currently selected text and update the cursor position.
    pub fn delete_selection<T: Text + ?Sized>(&mut self, text: &mut impl TextEditable<T>) {
        let first = self.first();
        let last = self.last();
        text.delete_range(first.idx(text.as_ref())..last.idx(text.as_ref()));
        if let Some(end) = self.end.take() {
            if self.start > end {
                self.start = end;
            }
        }
    }

    /// Handle moving the cursor with the given key.
    pub fn handle_input<T: Text + ?Sized>(
        &mut self,
        code: &Code,
        key: &Key,
        modifiers: &Modifiers,
        text: &mut impl TextEditable<T>,
        max_text_length: usize,
    ) {
        use Code::*;
        match code {
            ArrowUp => {
                self.move_cursor(
                    |c| c.up(text.as_ref()),
                    modifiers.contains(Modifiers::SHIFT),
                );
            }
            ArrowDown => {
                self.move_cursor(
                    |c| c.down(text.as_ref()),
                    modifiers.contains(Modifiers::SHIFT),
                );
            }
            ArrowRight => {
                if modifiers.contains(Modifiers::CONTROL) {
                    self.move_cursor(
                        |c| {
                            let mut change = 1;
                            let idx = c.idx(text.as_ref());
                            let length = text.as_ref().length();
                            while idx + change < length {
                                let chr = text.as_ref().character(idx + change).unwrap();
                                if chr.is_whitespace() {
                                    break;
                                }
                                change += 1;
                            }
                            c.move_col(change as i32, text.as_ref());
                        },
                        modifiers.contains(Modifiers::SHIFT),
                    );
                } else {
                    self.move_cursor(
                        |c| c.right(text.as_ref()),
                        modifiers.contains(Modifiers::SHIFT),
                    );
                }
            }
            ArrowLeft => {
                if modifiers.contains(Modifiers::CONTROL) {
                    self.move_cursor(
                        |c| {
                            let mut change = -1;
                            let idx = c.idx(text.as_ref()) as i32;
                            while idx + change > 0 {
                                let chr = text.as_ref().character((idx + change) as usize).unwrap();
                                if chr == ' ' {
                                    break;
                                }
                                change -= 1;
                            }
                            c.move_col(change, text.as_ref());
                        },
                        modifiers.contains(Modifiers::SHIFT),
                    );
                } else {
                    self.move_cursor(
                        |c| c.left(text.as_ref()),
                        modifiers.contains(Modifiers::SHIFT),
                    );
                }
            }
            End => {
                self.move_cursor(
                    |c| c.col = c.len_line(text.as_ref()),
                    modifiers.contains(Modifiers::SHIFT),
                );
            }
            Home => {
                self.move_cursor(|c| c.col = 0, modifiers.contains(Modifiers::SHIFT));
            }
            Backspace => {
                self.start.realize_col(text.as_ref());
                let mut start_idx = self.start.idx(text.as_ref());
                if self.end.is_some() {
                    self.delete_selection(text);
                } else if start_idx > 0 {
                    self.start.left(text.as_ref());
                    text.delete_range(start_idx - 1..start_idx);
                    if modifiers.contains(Modifiers::CONTROL) {
                        start_idx = self.start.idx(text.as_ref());
                        while start_idx > 0
                            && text
                                .as_ref()
                                .character(start_idx - 1)
                                .filter(|c| *c != ' ')
                                .is_some()
                        {
                            self.start.left(text.as_ref());
                            text.delete_range(start_idx - 1..start_idx);
                            start_idx = self.start.idx(text.as_ref());
                        }
                    }
                }
            }
            Enter => {
                if text.as_ref().length() + 1 - self.selection_len(text.as_ref()) <= max_text_length
                {
                    text.insert_character(self.start.idx(text.as_ref()), '\n');
                    self.start.col = 0;
                    self.start.down(text.as_ref());
                }
            }
            Tab => {
                if text.as_ref().length() + 1 - self.selection_len(text.as_ref()) <= max_text_length
                {
                    self.start.realize_col(text.as_ref());
                    self.delete_selection(text);
                    text.insert_character(self.start.idx(text.as_ref()), '\t');
                    self.start.right(text.as_ref());
                }
            }
            _ => {
                self.start.realize_col(text.as_ref());
                if let Key::Character(character) = key {
                    if text.as_ref().length() + 1 - self.selection_len(text.as_ref())
                        <= max_text_length
                    {
                        self.delete_selection(text);
                        let character = character.chars().next().unwrap();
                        text.insert_character(self.start.idx(text.as_ref()), character);
                        self.start.right(text.as_ref());
                    }
                }
            }
        }
    }

    /// Modify the end selection position
    pub fn with_end(&mut self, f: impl FnOnce(&mut Pos)) {
        let mut new = self.end.take().unwrap_or_else(|| self.start.clone());
        f(&mut new);
        self.end.replace(new);
    }

    /// Returns first position of the selection (this could be the start or the end depending on the position)
    pub fn first(&self) -> &Pos {
        if let Some(e) = &self.end {
            e.min(&self.start)
        } else {
            &self.start
        }
    }

    /// Returns last position of the selection (this could be the start or the end depending on the position)
    pub fn last(&self) -> &Pos {
        if let Some(e) = &self.end {
            e.max(&self.start)
        } else {
            &self.start
        }
    }

    /// Returns the length of the selection
    pub fn selection_len(&self, text: &(impl Text + ?Sized)) -> usize {
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
            &keyboard_types::Code::ArrowRight,
            &keyboard_types::Key::ArrowRight,
            &Modifiers::empty(),
            &mut text,
            10,
        );
    }

    for _ in 0..5 {
        cursor.handle_input(
            &keyboard_types::Code::Backspace,
            &keyboard_types::Key::Backspace,
            &Modifiers::empty(),
            &mut text,
            10,
        );
    }

    assert_eq!(text, " world\nhi");

    let goal_text = "hello world\nhi";
    let max_width = goal_text.len();
    cursor.handle_input(
        &keyboard_types::Code::KeyH,
        &keyboard_types::Key::Character("h".to_string()),
        &Modifiers::empty(),
        &mut text,
        max_width,
    );

    cursor.handle_input(
        &keyboard_types::Code::KeyE,
        &keyboard_types::Key::Character("e".to_string()),
        &Modifiers::empty(),
        &mut text,
        max_width,
    );

    cursor.handle_input(
        &keyboard_types::Code::KeyL,
        &keyboard_types::Key::Character("l".to_string()),
        &Modifiers::empty(),
        &mut text,
        max_width,
    );

    cursor.handle_input(
        &keyboard_types::Code::KeyL,
        &keyboard_types::Key::Character("l".to_string()),
        &Modifiers::empty(),
        &mut text,
        max_width,
    );

    cursor.handle_input(
        &keyboard_types::Code::KeyO,
        &keyboard_types::Key::Character("o".to_string()),
        &Modifiers::empty(),
        &mut text,
        max_width,
    );

    // these should be ignored
    for _ in 0..10 {
        cursor.handle_input(
            &keyboard_types::Code::KeyO,
            &keyboard_types::Key::Character("o".to_string()),
            &Modifiers::empty(),
            &mut text,
            max_width,
        );
    }

    assert_eq!(text.to_string(), goal_text);
}
