use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier},
    widgets::Widget,
};

use crate::{
    style::{convert, RinkColor, RinkStyle},
    Config,
};

pub struct RinkBuffer<'a> {
    buf: &'a mut Buffer,
    cfg: Config,
}

impl<'a> RinkBuffer<'a> {
    fn new(buf: &'a mut Buffer, cfg: Config) -> RinkBuffer<'a> {
        Self { buf, cfg }
    }

    pub fn set(&mut self, x: u16, y: u16, new: RinkCell) {
        let area = self.buf.area();
        if x < area.x || x >= area.width + area.x || y < area.y || y >= area.height + area.y {
            // panic!("({x}, {y}) is not in {area:?}");
            return;
        }
        let cell = self.buf.get_mut(x, y);
        cell.bg = convert(self.cfg.rendering_mode, new.bg.blend(cell.bg));
        if new.symbol.is_empty() {
            if !cell.symbol.is_empty() {
                // allows text to "shine through" transparent backgrounds
                cell.fg = convert(self.cfg.rendering_mode, new.bg.blend(cell.fg));
            }
        } else {
            cell.modifier = new.modifier;
            cell.symbol = new.symbol;
            cell.fg = convert(self.cfg.rendering_mode, new.fg.blend(cell.bg));
        }
    }
}

pub trait RinkWidget {
    fn render(self, area: Rect, buf: RinkBuffer);
}

pub struct WidgetWithContext<T: RinkWidget> {
    widget: T,
    config: Config,
}

impl<T: RinkWidget> WidgetWithContext<T> {
    pub fn new(widget: T, config: Config) -> WidgetWithContext<T> {
        WidgetWithContext { widget, config }
    }
}

impl<T: RinkWidget> Widget for WidgetWithContext<T> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        self.widget.render(area, RinkBuffer::new(buf, self.config));
    }
}

#[derive(Clone)]
pub struct RinkCell {
    pub symbol: String,
    pub bg: RinkColor,
    pub fg: RinkColor,
    pub modifier: Modifier,
}

impl Default for RinkCell {
    fn default() -> Self {
        Self {
            symbol: "".to_string(),
            fg: RinkColor {
                color: Color::Rgb(0, 0, 0),
                alpha: 0,
            },
            bg: RinkColor {
                color: Color::Rgb(0, 0, 0),
                alpha: 0,
            },
            modifier: Modifier::empty(),
        }
    }
}

impl RinkCell {
    pub fn set_style(&mut self, style: RinkStyle) {
        if let Some(c) = style.fg {
            self.fg = c;
        }
        if let Some(c) = style.bg {
            self.bg = c;
        }
        self.modifier = style.add_modifier;
        self.modifier.remove(style.sub_modifier);
    }
}
