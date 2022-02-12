/*
- [ ] pub display: Display,
- [x] pub position_type: PositionType,  --> kinda, stretch doesnt support everything
- [ ] pub direction: Direction,

- [x] pub flex_direction: FlexDirection,
- [x] pub flex_wrap: FlexWrap,
- [x] pub flex_grow: f32,
- [x] pub flex_shrink: f32,
- [x] pub flex_basis: Dimension,

- [x] pub overflow: Overflow, ---> kinda implemented... stretch doesnt have support for directional overflow

- [x] pub align_items: AlignItems,
- [x] pub align_self: AlignSelf,
- [x] pub align_content: AlignContent,

- [x] pub margin: Rect<Dimension>,
- [x] pub padding: Rect<Dimension>,

- [x] pub justify_content: JustifyContent,
- [ ] pub position: Rect<Dimension>,
- [ ] pub border: Rect<Dimension>,

- [ ] pub size: Size<Dimension>, ----> ??? seems to only be relevant for input?
- [ ] pub min_size: Size<Dimension>,
- [ ] pub max_size: Size<Dimension>,

- [ ] pub aspect_ratio: Number,
*/

use stretch2::{prelude::*, style::PositionType, style::Style};
use tui::style::{Color, Style as TuiStyle};

pub struct StyleModifer {
    pub style: Style,
    pub tui_style: TuiStyle,
    pub tui_modifier: TuiModifier,
}

pub struct TuiModifier {
    // border arrays start at the top and proceed clockwise
    pub border_colors: [Option<Color>; 4],
    pub border_types: [BorderType; 4],
    pub border_widths: [UnitSystem; 4],
    pub border_radi: [UnitSystem; 4],
}

#[derive(Clone, Copy)]
pub enum BorderType {
    DOTTED,
    DASHED,
    SOLID,
    DOUBLE,
    GROOVE,
    RIDGE,
    INSET,
    OUTSET,
    HIDDEN,
    NONE,
}

impl Default for TuiModifier {
    fn default() -> Self {
        Self {
            border_colors: [None; 4],
            border_types: [BorderType::NONE; 4],
            border_widths: [UnitSystem::Point(0.0); 4],
            border_radi: [UnitSystem::Point(0.0); 4],
        }
    }
}

fn parse_color(color: &str) -> Option<tui::style::Color> {
    match color {
        "red" => Some(Color::Red),
        "green" => Some(Color::Green),
        "blue" => Some(Color::Blue),
        "yellow" => Some(Color::Yellow),
        "cyan" => Some(Color::Cyan),
        "magenta" => Some(Color::Magenta),
        "white" => Some(Color::White),
        "black" => Some(Color::Black),
        _ => {
            if color.len() == 7 && color.starts_with('#') {
                let mut values = [0, 0, 0];
                let mut color_ok = true;
                for i in 0..values.len() {
                    if let Ok(v) = u8::from_str_radix(&color[(1 + 2 * i)..(1 + 2 * (i + 1))], 16) {
                        values[i] = v;
                    } else {
                        color_ok = false;
                    }
                }
                if color_ok {
                    Some(Color::Rgb(values[0], values[1], values[2]))
                } else {
                    None
                }
            } else if color.starts_with("rgb(") {
                let mut values = [0, 0, 0];
                let mut color_ok = true;
                for (v, i) in color[4..]
                    .trim_end_matches(')')
                    .split(',')
                    .zip(0..values.len())
                {
                    if let Ok(v) = v.trim().parse() {
                        values[i] = v;
                    } else {
                        color_ok = false;
                    }
                }
                if color_ok {
                    Some(Color::Rgb(values[0], values[1], values[2]))
                } else {
                    None
                }
            } else if color.starts_with("hsl(") {
                let mut values = [0, 0, 0];
                let mut color_ok = true;
                for (v, i) in color[4..]
                    .trim_end_matches(')')
                    .split(',')
                    .zip(0..values.len())
                {
                    if let Ok(v) = v.trim_end_matches('%').trim().parse() {
                        values[i] = v;
                    } else {
                        color_ok = false;
                    }
                }
                if color_ok {
                    let [h, s, l] = [
                        values[0] as f32 / 360.0,
                        values[1] as f32 / 100.0,
                        values[2] as f32 / 100.0,
                    ];
                    let rgb = if s == 0.0 {
                        [l as u8; 3]
                    } else {
                        fn hue_to_rgb(p: f32, q: f32, mut t: f32) -> f32 {
                            if t < 0.0 {
                                t += 1.0;
                            }
                            if t > 1.0 {
                                t -= 1.0;
                            }
                            if t < 1.0 / 6.0 {
                                p + (q - p) * 6.0 * t
                            } else if t < 1.0 / 2.0 {
                                q
                            } else if t < 2.0 / 3.0 {
                                p + (q - p) * (2.0 / 3.0 - t) * 6.0
                            } else {
                                p
                            }
                        }

                        let q = if l < 0.5 {
                            l * (1.0 + s)
                        } else {
                            l + s - l * s
                        };
                        let p = 2.0 * l - q;
                        [
                            (hue_to_rgb(p, q, h + 1.0 / 3.0) * 255.0) as u8,
                            (hue_to_rgb(p, q, h) * 255.0) as u8,
                            (hue_to_rgb(p, q, h - 1.0 / 3.0) * 255.0) as u8,
                        ]
                    };

                    Some(Color::Rgb(rgb[0], rgb[1], rgb[2]))
                } else {
                    None
                }
            } else {
                None
            }
        }
    }
}

/// applies the entire html namespace defined in dioxus-html
pub fn apply_attributes(
    //
    name: &str,
    value: &str,
    style: &mut StyleModifer,
) {
    match name {
        "align-content"
        | "align-items"
        | "align-self" => apply_align(name, value, style),

        "animation"
        | "animation-delay"
        | "animation-direction"
        | "animation-duration"
        | "animation-fill-mode"
        | "animation-iteration-count"
        | "animation-name"
        | "animation-play-state"
        | "animation-timing-function" => apply_animation(name, value, style),

        "backface-visibility" => {}

        "background"
        | "background-attachment"
        | "background-clip"
        | "background-color"
        | "background-image"
        | "background-origin"
        | "background-position"
        | "background-repeat"
        | "background-size" => apply_background(name, value, style),

        "border"
        | "border-bottom"
        | "border-bottom-color"
        | "border-bottom-left-radius"
        | "border-bottom-right-radius"
        | "border-bottom-style"
        | "border-bottom-width"
        | "border-collapse"
        | "border-color"
        | "border-image"
        | "border-image-outset"
        | "border-image-repeat"
        | "border-image-slice"
        | "border-image-source"
        | "border-image-width"
        | "border-left"
        | "border-left-color"
        | "border-left-style"
        | "border-left-width"
        | "border-radius"
        | "border-right"
        | "border-right-color"
        | "border-right-style"
        | "border-right-width"
        | "border-spacing"
        | "border-style"
        | "border-top"
        | "border-top-color"
        | "border-top-left-radius"
        | "border-top-right-radius"
        | "border-top-style"
        | "border-top-width"
        | "border-width" => apply_border(name, value, style),

        "bottom" => {}
        "box-shadow" => {}
        "box-sizing" => {}
        "caption-side" => {}
        "clear" => {}
        "clip" => {}

        "color" => {
            if let Some(c) = parse_color(value) {
                style.tui_style.fg.replace(c);
            }
        }

        "column-count"
        | "column-fill"
        | "column-gap"
        | "column-rule"
        | "column-rule-color"
        | "column-rule-style"
        | "column-rule-width"
        | "column-span"
        // add column-width
        | "column-width" => apply_column(name, value, style),

        "columns" => {}

        "content" => {}
        "counter-increment" => {}
        "counter-reset" => {}

        "cursor" => {}
        "direction" => {
            match value {
                "ltr" => style.style.direction = Direction::LTR,
                "rtl" => style.style.direction = Direction::RTL,
                _ => {}
            }
        }

        "display" => apply_display(name, value, style),

        "empty-cells" => {}

        "flex"
        | "flex-basis"
        | "flex-direction"
        | "flex-flow"
        | "flex-grow"
        | "flex-shrink"
        | "flex-wrap" => apply_flex(name, value, style),

        "float" => {}

        "font"
        | "font-family"
        | "font-size"
        | "font-size-adjust"
        | "font-stretch"
        | "font-style"
        | "font-variant"
        | "font-weight" => apply_font(name, value, style),

        "height" => {
            if let Some(v) = parse_value(value){
                style.style.size.height = match v {
                    UnitSystem::Percent(v)=> Dimension::Percent(v/100.0),
                    UnitSystem::Point(v)=> Dimension::Points(v),
                };
            }
        }
        "justify-content" => {
            use JustifyContent::*;
            style.style.justify_content = match value {
                "flex-start" => FlexStart,
                "flex-end" => FlexEnd,
                "center" => Center,
                "space-between" => SpaceBetween,
                "space-around" => SpaceAround,
                "space-evenly" => SpaceEvenly,
                _ => FlexStart,
            };
        }
        "left" => {}
        "letter-spacing" => {}
        "line-height" => {}

        "list-style"
        | "list-style-image"
        | "list-style-position"
        | "list-style-type" => {}

        "margin"
        | "margin-bottom"
        | "margin-left"
        | "margin-right"
        | "margin-top" => apply_margin(name, value, style),

        "max-height" => {}
        "max-width" => {}
        "min-height" => {}
        "min-width" => {}

        "opacity" => {}
        "order" => {}
        "outline" => {}

        "outline-color"
        | "outline-offset"
        | "outline-style"
        | "outline-width" => {}

        "overflow"
        | "overflow-x"
        | "overflow-y" => apply_overflow(name, value, style),

        "padding"
        | "padding-bottom"
        | "padding-left"
        | "padding-right"
        | "padding-top" => apply_padding(name, value, style),

        "page-break-after"
        | "page-break-before"
        | "page-break-inside" => {}

        "perspective"
        | "perspective-origin" => {}

        "position" => {
            match value {
                "static" => {}
                "relative" => style.style.position_type = PositionType::Relative,
                "fixed" => {}
                "absolute" => style.style.position_type = PositionType::Absolute,
                "sticky" => {}
                _ => {}
            }

        }

        "pointer-events" => {}

        "quotes" => {}
        "resize" => {}
        "right" => {}
        "tab-size" => {}
        "table-layout" => {}

        "text-align"
        | "text-align-last"
        | "text-decoration"
        | "text-decoration-color"
        | "text-decoration-line"
        | "text-decoration-style"
        | "text-indent"
        | "text-justify"
        | "text-overflow"
        | "text-shadow"
        | "text-transform" => apply_text(name, value, style),

        "top" => {}

        "transform"
        | "transform-origin"
        | "transform-style" => apply_transform(name, value, style),

        "transition"
        | "transition-delay"
        | "transition-duration"
        | "transition-property"
        | "transition-timing-function" => apply_transition(name, value, style),

        "vertical-align" => {}
        "visibility" => {}
        "white-space" => {}
        "width" => {
            if let Some(v) = parse_value(value){
                style.style.size.width = match v {
                    UnitSystem::Percent(v)=> Dimension::Percent(v/100.0),
                    UnitSystem::Point(v)=> Dimension::Points(v),
                };
            }
        }
        "word-break" => {}
        "word-spacing" => {}
        "word-wrap" => {}
        "z-index" => {}
        _ => {}
    }
}

#[derive(Clone, Copy)]
pub enum UnitSystem {
    Percent(f32),
    Point(f32),
}

fn parse_value(value: &str) -> Option<UnitSystem> {
    if value.ends_with("px") {
        if let Ok(px) = value.trim_end_matches("px").parse::<f32>() {
            Some(UnitSystem::Point(px))
        } else {
            None
        }
    } else if value.ends_with("%") {
        if let Ok(pct) = value.trim_end_matches("%").parse::<f32>() {
            Some(UnitSystem::Percent(pct))
        } else {
            None
        }
    } else {
        None
    }
}

fn apply_overflow(name: &str, value: &str, style: &mut StyleModifer) {
    match name {
        // todo: add more overflow support to stretch2
        "overflow" | "overflow-x" | "overflow-y" => {
            style.style.overflow = match value {
                "auto" => Overflow::Visible,
                "hidden" => Overflow::Hidden,
                "scroll" => Overflow::Scroll,
                "visible" => Overflow::Visible,
                _ => Overflow::Visible,
            };
        }
        _ => {}
    }
}

fn apply_display(_name: &str, value: &str, style: &mut StyleModifer) {
    style.style.display = match value {
        "flex" => Display::Flex,
        "block" => Display::None,
        _ => Display::Flex,
    }

    // TODO: there are way more variants
    // stretch needs to be updated to handle them
    //
    // "block" => Display::Block,
    // "inline" => Display::Inline,
    // "inline-block" => Display::InlineBlock,
    // "inline-table" => Display::InlineTable,
    // "list-item" => Display::ListItem,
    // "run-in" => Display::RunIn,
    // "table" => Display::Table,
    // "table-caption" => Display::TableCaption,
    // "table-cell" => Display::TableCell,
    // "table-column" => Display::TableColumn,
    // "table-column-group" => Display::TableColumnGroup,
    // "table-footer-group" => Display::TableFooterGroup,
    // "table-header-group" => Display::TableHeaderGroup,
    // "table-row" => Display::TableRow,
    // "table-row-group" => Display::TableRowGroup,
    // "none" => Display::None,
    // _ => Display::Inline,
}

fn apply_background(name: &str, value: &str, style: &mut StyleModifer) {
    match name {
        "background-color" => {
            if let Some(c) = parse_color(value) {
                style.tui_style.bg.replace(c);
            }
        }
        "background" => {}
        "background-attachment" => {}
        "background-clip" => {}
        "background-image" => {}
        "background-origin" => {}
        "background-position" => {}
        "background-repeat" => {}
        "background-size" => {}
        _ => {}
    }
}

fn apply_border(name: &str, value: &str, style: &mut StyleModifer) {
    fn parse_border_type(v: &str) -> BorderType {
        match v {
            "dotted" => BorderType::DOTTED,
            "dashed" => BorderType::DASHED,
            "solid" => BorderType::SOLID,
            "double" => BorderType::DOUBLE,
            "groove" => BorderType::GROOVE,
            "ridge" => BorderType::RIDGE,
            "inset" => BorderType::INSET,
            "outset" => BorderType::OUTSET,
            "none" => BorderType::NONE,
            "hidden" => BorderType::HIDDEN,
            _ => todo!(),
        }
    }
    match name {
        "border" => {}
        "border-bottom" => {}
        "border-bottom-color" => {
            if let Some(c) = parse_color(value) {
                style.tui_modifier.border_colors[2] = Some(c);
            }
        }
        "border-bottom-left-radius" => {
            if let Some(v) = parse_value(value) {
                style.tui_modifier.border_radi[2] = v;
            }
        }
        "border-bottom-right-radius" => {
            if let Some(v) = parse_value(value) {
                style.tui_modifier.border_radi[1] = v;
            }
        }
        "border-bottom-style" => style.tui_modifier.border_types[2] = parse_border_type(value),
        "border-bottom-width" => {
            if let Some(v) = parse_value(value) {
                style.tui_modifier.border_widths[2] = v;
            }
        }
        "border-collapse" => {}
        "border-color" => {
            let values: Vec<_> = value.split(' ').collect();
            if values.len() == 1 {
                if let Some(c) = parse_color(values[0]) {
                    for i in 0..4 {
                        style.tui_modifier.border_colors[i] = Some(c);
                    }
                }
            } else {
                for (i, v) in values.into_iter().enumerate() {
                    if let Some(c) = parse_color(v) {
                        style.tui_modifier.border_colors[i] = Some(c);
                    }
                }
            }
        }
        "border-image" => {}
        "border-image-outset" => {}
        "border-image-repeat" => {}
        "border-image-slice" => {}
        "border-image-source" => {}
        "border-image-width" => {}
        "border-left" => {}
        "border-left-color" => {
            if let Some(c) = parse_color(value) {
                style.tui_modifier.border_colors[3] = Some(c);
            }
        }
        "border-left-style" => style.tui_modifier.border_types[3] = parse_border_type(value),
        "border-left-width" => {
            if let Some(v) = parse_value(value) {
                style.tui_modifier.border_widths[3] = v;
            }
        }
        "border-radius" => {
            let values: Vec<_> = value.split(' ').collect();
            if values.len() == 1 {
                if let Some(r) = parse_value(values[0]) {
                    for i in 0..4 {
                        style.tui_modifier.border_radi[i] = r;
                    }
                }
            } else {
                for (i, v) in values.into_iter().enumerate() {
                    if let Some(r) = parse_value(v) {
                        style.tui_modifier.border_radi[i] = r;
                    }
                }
            }
        }
        "border-right" => {}
        "border-right-color" => {
            if let Some(c) = parse_color(value) {
                style.tui_modifier.border_colors[1] = Some(c);
            }
        }
        "border-right-style" => style.tui_modifier.border_types[1] = parse_border_type(value),
        "border-right-width" => {
            if let Some(v) = parse_value(value) {
                style.tui_modifier.border_widths[1] = v;
            }
        }
        "border-spacing" => {}
        "border-style" => {
            let values: Vec<_> = value.split(' ').collect();
            if values.len() == 1 {
                let border = parse_border_type(values[0]);
                for i in 0..4 {
                    style.tui_modifier.border_types[i] = border;
                }
            } else {
                for (i, v) in values.into_iter().enumerate() {
                    style.tui_modifier.border_types[i] = parse_border_type(v);
                }
            }
        }
        "border-top" => {}
        "border-top-color" => {
            if let Some(c) = parse_color(value) {
                style.tui_modifier.border_colors[0] = Some(c);
            }
        }
        "border-top-left-radius" => {
            if let Some(v) = parse_value(value) {
                style.tui_modifier.border_radi[3] = v;
            }
        }
        "border-top-right-radius" => {
            if let Some(v) = parse_value(value) {
                style.tui_modifier.border_radi[0] = v;
            }
        }
        "border-top-style" => style.tui_modifier.border_types[0] = parse_border_type(value),
        "border-top-width" => {
            if let Some(v) = parse_value(value) {
                style.tui_modifier.border_widths[0] = v;
            }
        }
        "border-width" => {
            if let Some(v) = parse_value(value) {
                for i in 0..4 {
                    style.tui_modifier.border_widths[i] = v;
                }
            }
        }
        _ => (),
    }
}

fn apply_animation(name: &str, _value: &str, _style: &mut StyleModifer) {
    match name {
        "animation" => {}
        "animation-delay" => {}
        "animation-direction =>{}" => {}
        "animation-duration" => {}
        "animation-fill-mode" => {}
        "animation-itera =>{}tion-count" => {}
        "animation-name" => {}
        "animation-play-state" => {}
        "animation-timing-function" => {}
        _ => {}
    }
}

fn apply_column(name: &str, _value: &str, _style: &mut StyleModifer) {
    match name {
        "column-count" => {}
        "column-fill" => {}
        "column-gap" => {}
        "column-rule" => {}
        "column-rule-color" => {}
        "column-rule-style" => {}
        "column-rule-width" => {}
        "column-span" => {}
        "column-width" => {}
        _ => {}
    }
}

fn apply_flex(name: &str, value: &str, style: &mut StyleModifer) {
    // - [x] pub flex_direction: FlexDirection,
    // - [x] pub flex_wrap: FlexWrap,
    // - [x] pub flex_grow: f32,
    // - [x] pub flex_shrink: f32,
    // - [x] pub flex_basis: Dimension,

    match name {
        "flex" => {}
        "flex-direction" => {
            use FlexDirection::*;
            style.style.flex_direction = match value {
                "row" => Row,
                "row-reverse" => RowReverse,
                "column" => Column,
                "column-reverse" => ColumnReverse,
                _ => Row,
            };
        }
        "flex-basis" => {
            if let Some(v) = parse_value(value) {
                style.style.flex_basis = match v {
                    UnitSystem::Percent(v) => Dimension::Percent(v / 100.0),
                    UnitSystem::Point(v) => Dimension::Points(v),
                };
            }
        }
        "flex-flow" => {}
        "flex-grow" => {
            if let Ok(val) = value.parse::<f32>() {
                style.style.flex_grow = val;
            }
        }
        "flex-shrink" => {
            if let Ok(px) = value.parse::<f32>() {
                style.style.flex_shrink = px;
            }
        }
        "flex-wrap" => {
            use FlexWrap::*;
            style.style.flex_wrap = match value {
                "nowrap" => NoWrap,
                "wrap" => Wrap,
                "wrap-reverse" => WrapReverse,
                _ => NoWrap,
            };
        }
        _ => {}
    }
}

fn apply_font(name: &str, value: &str, style: &mut StyleModifer) {
    use tui::style::Modifier;
    match name {
        "font" => todo!(),
        "font-family" => todo!(),
        "font-size" => todo!(),
        "font-size-adjust" => todo!(),
        "font-stretch" => todo!(),
        "font-style" => match value {
            "italic" => style.tui_style = style.tui_style.add_modifier(Modifier::ITALIC),
            "oblique" => style.tui_style = style.tui_style.add_modifier(Modifier::ITALIC),
            _ => (),
        },
        "font-variant" => todo!(),
        "font-weight" => match value {
            "bold" => style.tui_style = style.tui_style.add_modifier(Modifier::BOLD),
            "normal" => style.tui_style = style.tui_style.remove_modifier(Modifier::BOLD),
            _ => (),
        },
        _ => (),
    }
}

fn apply_padding(name: &str, value: &str, style: &mut StyleModifer) {
    match parse_value(value) {
        Some(UnitSystem::Percent(v)) => match name {
            "padding" => {
                let v = Dimension::Percent(v / 100.0);
                style.style.padding.top = v;
                style.style.padding.bottom = v;
                style.style.padding.start = v;
                style.style.padding.end = v;
            }
            "padding-bottom" => style.style.padding.bottom = Dimension::Percent(v / 100.0),
            "padding-left" => style.style.padding.start = Dimension::Percent(v / 100.0),
            "padding-right" => style.style.padding.end = Dimension::Percent(v / 100.0),
            "padding-top" => style.style.padding.top = Dimension::Percent(v / 100.0),
            _ => {}
        },
        Some(UnitSystem::Point(v)) => match name {
            "padding" => {
                style.style.padding.top = Dimension::Points(v);
                style.style.padding.bottom = Dimension::Points(v);
                style.style.padding.start = Dimension::Points(v);
                style.style.padding.end = Dimension::Points(v);
            }
            "padding-bottom" => style.style.padding.bottom = Dimension::Points(v),
            "padding-left" => style.style.padding.start = Dimension::Points(v),
            "padding-right" => style.style.padding.end = Dimension::Points(v),
            "padding-top" => style.style.padding.top = Dimension::Points(v),
            _ => {}
        },
        None => {}
    }
}

fn apply_text(name: &str, value: &str, style: &mut StyleModifer) {
    use tui::style::Modifier;

    match name {
        "text-align" => todo!(),
        "text-align-last" => todo!(),
        "text-decoration" | "text-decoration-line" => {
            for v in value.split(' ') {
                match v {
                    "line-through" => {
                        style.tui_style = style.tui_style.add_modifier(Modifier::CROSSED_OUT)
                    }
                    "underline" => {
                        style.tui_style = style.tui_style.add_modifier(Modifier::UNDERLINED)
                    }
                    _ => (),
                }
            }
        }
        "text-decoration-color" => todo!(),
        "text-decoration-style" => todo!(),
        "text-indent" => todo!(),
        "text-justify" => todo!(),
        "text-overflow" => todo!(),
        "text-shadow" => todo!(),
        "text-transform" => todo!(),
        _ => todo!(),
    }
}

fn apply_transform(_name: &str, _value: &str, _style: &mut StyleModifer) {
    todo!()
}

fn apply_transition(_name: &str, _value: &str, _style: &mut StyleModifer) {
    todo!()
}

fn apply_align(name: &str, value: &str, style: &mut StyleModifer) {
    match name {
        "align-items" => {
            use AlignItems::*;
            style.style.align_items = match value {
                "flex-start" => FlexStart,
                "flex-end" => FlexEnd,
                "center" => Center,
                "baseline" => Baseline,
                "stretch" => Stretch,
                _ => FlexStart,
            };
        }
        "align-content" => {
            use AlignContent::*;
            style.style.align_content = match value {
                "flex-start" => FlexStart,
                "flex-end" => FlexEnd,
                "center" => Center,
                "space-between" => SpaceBetween,
                "space-around" => SpaceAround,
                _ => FlexStart,
            };
        }
        "align-self" => {
            use AlignSelf::*;
            style.style.align_self = match value {
                "auto" => Auto,
                "flex-start" => FlexStart,
                "flex-end" => FlexEnd,
                "center" => Center,
                "baseline" => Baseline,
                "stretch" => Stretch,
                _ => Auto,
            };
        }
        _ => {}
    }
}

pub fn apply_size(_name: &str, _value: &str, _style: &mut StyleModifer) {
    //
}

pub fn apply_margin(name: &str, value: &str, style: &mut StyleModifer) {
    match parse_value(value) {
        Some(UnitSystem::Percent(v)) => match name {
            "margin" => {
                let v = Dimension::Percent(v / 100.0);
                style.style.margin.top = v;
                style.style.margin.bottom = v;
                style.style.margin.start = v;
                style.style.margin.end = v;
            }
            "margin-top" => style.style.margin.top = Dimension::Percent(v / 100.0),
            "margin-bottom" => style.style.margin.bottom = Dimension::Percent(v / 100.0),
            "margin-left" => style.style.margin.start = Dimension::Percent(v / 100.0),
            "margin-right" => style.style.margin.end = Dimension::Percent(v / 100.0),
            _ => {}
        },
        Some(UnitSystem::Point(v)) => match name {
            "margin" => {
                style.style.margin.top = Dimension::Points(v);
                style.style.margin.bottom = Dimension::Points(v);
                style.style.margin.start = Dimension::Points(v);
                style.style.margin.end = Dimension::Points(v);
            }
            "margin-top" => style.style.margin.top = Dimension::Points(v),
            "margin-bottom" => style.style.margin.bottom = Dimension::Points(v),
            "margin-left" => style.style.margin.start = Dimension::Points(v),
            "margin-right" => style.style.margin.end = Dimension::Points(v),
            _ => {}
        },
        None => {}
    }
}
