use ratatui::prelude::*;
use std::fmt::{self, Display, Formatter};

/// A buffer that can be rendered to and then dumped as raw ansi codes
///
/// This is taken from a PR on the ratatui repo (https://github.com/ratatui/ratatui/pull/1065) and
/// modified to be more appropriate for our use case.
pub struct AnsiStringLine {
    buf: Buffer,
}

// The sentinel character used to mark the end of the ansi string so when we dump it, we know where to stop
// Not sure if we actually still need this....
const SENTINEL: &str = "✆";

impl AnsiStringLine {
    /// Creates a new `AnsiStringBuffer` with the given width and height.
    pub(crate) fn new(width: u16) -> Self {
        Self {
            buf: Buffer::empty(Rect::new(0, 0, width, 1)),
        }
    }

    /// Renders the given widget to the buffer, returning the string with the ansi codes
    pub(crate) fn render(mut self, widget: impl Widget) -> String {
        widget.render(self.buf.area, &mut self.buf);
        self.trim_end();
        self.to_string()
    }

    /// Trims the buffer to the last line, returning the number of cells to be rendered
    #[allow(deprecated)]
    fn trim_end(&mut self) {
        for y in 0..self.buf.area.height {
            let start_x = self.buf.area.width;
            let mut first_non_empty = start_x;
            for x in (0..start_x).rev() {
                if self.buf.get(x, y) != &buffer::Cell::EMPTY {
                    break;
                }
                first_non_empty = x;
            }

            if first_non_empty != start_x {
                self.buf.get_mut(first_non_empty, y).set_symbol(SENTINEL);
            }
        }
    }

    fn write_fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut last_style = None;
        for y in 0..self.buf.area.height {
            for x in 0..self.buf.area.width {
                let cell = self.buf.cell((x, y)).unwrap();
                if cell.symbol() == SENTINEL {
                    break;
                }

                let style = (cell.fg, cell.bg, cell.modifier);
                if last_style.is_none() || last_style != Some(style) {
                    write_cell_style(f, cell)?;
                    last_style = Some(style);
                }
                f.write_str(cell.symbol())?;
            }
        }
        f.write_str("\u{1b}[0m")
    }
}

impl Display for AnsiStringLine {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.write_fmt(f)
    }
}

fn write_cell_style(f: &mut Formatter, cell: &buffer::Cell) -> fmt::Result {
    f.write_str("\u{1b}[")?;
    write_modifier(f, cell.modifier)?;
    write_fg(f, cell.fg)?;
    write_bg(f, cell.bg)?;
    f.write_str("m")
}

fn write_modifier(f: &mut Formatter, modifier: Modifier) -> fmt::Result {
    if modifier.contains(Modifier::BOLD) {
        f.write_str("1;")?;
    }
    if modifier.contains(Modifier::DIM) {
        f.write_str("2;")?;
    }
    if modifier.contains(Modifier::ITALIC) {
        f.write_str("3;")?;
    }
    if modifier.contains(Modifier::UNDERLINED) {
        f.write_str("4;")?;
    }
    if modifier.contains(Modifier::SLOW_BLINK) {
        f.write_str("5;")?;
    }
    if modifier.contains(Modifier::RAPID_BLINK) {
        f.write_str("6;")?;
    }
    if modifier.contains(Modifier::REVERSED) {
        f.write_str("7;")?;
    }
    if modifier.contains(Modifier::HIDDEN) {
        f.write_str("8;")?;
    }
    if modifier.contains(Modifier::CROSSED_OUT) {
        f.write_str("9;")?;
    }
    Ok(())
}

fn write_fg(f: &mut Formatter, color: Color) -> fmt::Result {
    f.write_str(match color {
        Color::Reset => "39",
        Color::Black => "30",
        Color::Red => "31",
        Color::Green => "32",
        Color::Yellow => "33",
        Color::Blue => "34",
        Color::Magenta => "35",
        Color::Cyan => "36",
        Color::Gray => "37",
        Color::DarkGray => "90",
        Color::LightRed => "91",
        Color::LightGreen => "92",
        Color::LightYellow => "93",
        Color::LightBlue => "94",
        Color::LightMagenta => "95",
        Color::LightCyan => "96",
        Color::White => "97",
        _ => "",
    })?;
    if let Color::Rgb(red, green, blue) = color {
        f.write_fmt(format_args!("38;2;{red};{green};{blue}"))?;
    }
    if let Color::Indexed(i) = color {
        f.write_fmt(format_args!("38;5;{i}"))?;
    }
    f.write_str(";")
}

fn write_bg(f: &mut Formatter, color: Color) -> fmt::Result {
    f.write_str(match color {
        Color::Reset => "49",
        Color::Black => "40",
        Color::Red => "41",
        Color::Green => "42",
        Color::Yellow => "43",
        Color::Blue => "44",
        Color::Magenta => "45",
        Color::Cyan => "46",
        Color::Gray => "47",
        Color::DarkGray => "100",
        Color::LightRed => "101",
        Color::LightGreen => "102",
        Color::LightYellow => "103",
        Color::LightBlue => "104",
        Color::LightMagenta => "105",
        Color::LightCyan => "106",
        Color::White => "107",
        _ => "",
    })?;
    if let Color::Rgb(red, green, blue) = color {
        f.write_fmt(format_args!("48;2;{red};{green};{blue}"))?;
    }
    if let Color::Indexed(i) = color {
        f.write_fmt(format_args!("48;5;{i}"))?;
    }
    Ok(())
}
