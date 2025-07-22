use ratatui::{buffer::Cell, prelude::*};
use std::fmt::{self, Write};

/// A buffer that can be rendered to and then dumped as raw ansi codes
///
/// This is taken from a PR on the ratatui repo (<https://github.com/ratatui/ratatui/pull/1065>) and
/// modified to be more appropriate for our use case.
pub fn ansi_string_to_line(line: Line) -> String {
    let graphemes = line.styled_graphemes(Style::default());
    let mut out = String::with_capacity(line.spans.first().map_or(0, |s| s.content.len() * 2));
    let mut last_style = None;
    for grapheme in graphemes {
        let mut cell = Cell::default();
        cell.set_symbol(grapheme.symbol);
        cell.set_style(grapheme.style);

        let style = (cell.fg, cell.bg, cell.modifier);
        if last_style.is_none() || last_style != Some(style) {
            _ = write_cell_style(&mut out, cell.modifier, cell.fg, cell.bg);
            last_style = Some(style);
        }

        _ = out.write_str(cell.symbol());
    }

    _ = out.write_str("\u{1b}[0m");

    out
}

fn write_cell_style(f: &mut impl Write, modifier: Modifier, fg: Color, bg: Color) -> fmt::Result {
    f.write_str("\u{1b}[")?;

    // Write the modifier codes
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

    // Write the foreground
    f.write_str(match fg {
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
    if let Color::Rgb(red, green, blue) = fg {
        f.write_fmt(format_args!("38;2;{red};{green};{blue}"))?;
    }
    if let Color::Indexed(i) = fg {
        f.write_fmt(format_args!("38;5;{i}"))?;
    }
    f.write_str(";")?;

    // Write the background
    f.write_str(match bg {
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
    if let Color::Rgb(red, green, blue) = bg {
        f.write_fmt(format_args!("48;2;{red};{green};{blue}"))?;
    }
    if let Color::Indexed(i) = bg {
        f.write_fmt(format_args!("48;5;{i}"))?;
    }

    f.write_str("m")
}
