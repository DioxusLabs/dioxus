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
- [x] pub border: Rect<Dimension>,

- [ ] pub size: Size<Dimension>, ----> ??? seems to only be relevant for input?
- [ ] pub min_size: Size<Dimension>,
- [ ] pub max_size: Size<Dimension>,

- [ ] pub aspect_ratio: Number,
*/

use stretch2::{prelude::*, style::PositionType};

/// applies the entire html namespace defined in dioxus-html
pub fn apply_layout_attributes(
    //
    name: &str,
    value: &str,
    style: &mut Style,
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
                "ltr" => style.direction = Direction::LTR,
                "rtl" => style.direction = Direction::RTL,
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

        "height" => {
            if let Some(v) = parse_value(value){
                style.size.height = match v {
                    UnitSystem::Percent(v)=> Dimension::Percent(v/100.0),
                    UnitSystem::Point(v)=> Dimension::Points(v),
                };
            }
        }
        "justify-content" => {
            use JustifyContent::*;
            style.justify_content = match value {
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
                "relative" => style.position_type = PositionType::Relative,
                "fixed" => {}
                "absolute" => style.position_type = PositionType::Absolute,
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
                style.size.width = match v {
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

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum UnitSystem {
    Percent(f32),
    Point(f32),
}

impl Into<Dimension> for UnitSystem {
    fn into(self) -> Dimension {
        match self {
            Self::Percent(v) => Dimension::Percent(v),
            Self::Point(v) => Dimension::Points(v),
        }
    }
}

pub fn parse_value(value: &str) -> Option<UnitSystem> {
    if value.ends_with("px") {
        if let Ok(px) = value.trim_end_matches("px").parse::<f32>() {
            Some(UnitSystem::Point(px))
        } else {
            None
        }
    } else if value.ends_with('%') {
        if let Ok(pct) = value.trim_end_matches('%').parse::<f32>() {
            Some(UnitSystem::Percent(pct))
        } else {
            None
        }
    } else {
        None
    }
}

fn apply_overflow(name: &str, value: &str, style: &mut Style) {
    match name {
        // todo: add more overflow support to stretch2
        "overflow" | "overflow-x" | "overflow-y" => {
            style.overflow = match value {
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

fn apply_display(_name: &str, value: &str, style: &mut Style) {
    style.display = match value {
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

fn apply_border(name: &str, value: &str, style: &mut Style) {
    match name {
        "border" => {}
        "border-bottom" => {}
        "border-bottom-color" => {}
        "border-bottom-left-radius" => {}
        "border-bottom-right-radius" => {}
        "border-bottom-style" => {
            if style.border.bottom == Dimension::default() {
                let v = Dimension::Points(1.0);
                style.border.bottom = v;
            }
        }
        "border-bottom-width" => {
            if let Some(v) = parse_value(value) {
                style.border.bottom = v.into();
            }
        }
        "border-collapse" => {}
        "border-color" => {}
        "border-image" => {}
        "border-image-outset" => {}
        "border-image-repeat" => {}
        "border-image-slice" => {}
        "border-image-source" => {}
        "border-image-width" => {}
        "border-left" => {}
        "border-left-color" => {}
        "border-left-style" => {
            if style.border.start == Dimension::default() {
                let v = Dimension::Points(1.0);
                style.border.start = v;
            }
        }
        "border-left-width" => {
            if let Some(v) = parse_value(value) {
                style.border.start = v.into();
            }
        }
        "border-radius" => {}
        "border-right" => {}
        "border-right-color" => {}
        "border-right-style" => {
            let v = Dimension::Points(1.0);
            style.border.end = v;
        }
        "border-right-width" => {
            if let Some(v) = parse_value(value) {
                style.border.end = v.into();
            }
        }
        "border-spacing" => {}
        "border-style" => {
            if style.border.top == Dimension::default() {
                let v = Dimension::Points(1.0);
                style.border.top = v;
            }
            if style.border.bottom == Dimension::default() {
                let v = Dimension::Points(1.0);
                style.border.bottom = v;
            }
            if style.border.start == Dimension::default() {
                let v = Dimension::Points(1.0);
                style.border.start = v;
            }
            if style.border.end == Dimension::default() {
                let v = Dimension::Points(1.0);
                style.border.end = v;
            }
        }
        "border-top" => {}
        "border-top-color" => {}
        "border-top-left-radius" => {}
        "border-top-right-radius" => {}
        "border-top-style" => {
            if style.border.top == Dimension::default() {
                let v = Dimension::Points(1.0);
                style.border.top = v;
            }
        }
        "border-top-width" => {
            if let Some(v) = parse_value(value) {
                style.border.top = v.into();
            }
        }
        "border-width" => {
            let values: Vec<_> = value.split(' ').collect();
            if values.len() == 1 {
                if let Some(w) = parse_value(values[0]) {
                    style.border.top = w.into();
                    style.border.bottom = w.into();
                    style.border.start = w.into();
                    style.border.end = w.into();
                }
            } else {
                let border_widths = [
                    &mut style.border.top,
                    &mut style.border.bottom,
                    &mut style.border.start,
                    &mut style.border.end,
                ];
                for (v, width) in values.into_iter().zip(border_widths) {
                    if let Some(w) = parse_value(v) {
                        *width = w.into();
                    }
                }
            }
        }
        _ => (),
    }
}

fn apply_animation(name: &str, _value: &str, _style: &mut Style) {
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

fn apply_column(name: &str, _value: &str, _style: &mut Style) {
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

fn apply_flex(name: &str, value: &str, style: &mut Style) {
    // - [x] pub flex_direction: FlexDirection,
    // - [x] pub flex_wrap: FlexWrap,
    // - [x] pub flex_grow: f32,
    // - [x] pub flex_shrink: f32,
    // - [x] pub flex_basis: Dimension,

    match name {
        "flex" => {}
        "flex-direction" => {
            use FlexDirection::*;
            style.flex_direction = match value {
                "row" => Row,
                "row-reverse" => RowReverse,
                "column" => Column,
                "column-reverse" => ColumnReverse,
                _ => Row,
            };
        }
        "flex-basis" => {
            if let Some(v) = parse_value(value) {
                style.flex_basis = match v {
                    UnitSystem::Percent(v) => Dimension::Percent(v / 100.0),
                    UnitSystem::Point(v) => Dimension::Points(v),
                };
            }
        }
        "flex-flow" => {}
        "flex-grow" => {
            if let Ok(val) = value.parse::<f32>() {
                style.flex_grow = val;
            }
        }
        "flex-shrink" => {
            if let Ok(px) = value.parse::<f32>() {
                style.flex_shrink = px;
            }
        }
        "flex-wrap" => {
            use FlexWrap::*;
            style.flex_wrap = match value {
                "nowrap" => NoWrap,
                "wrap" => Wrap,
                "wrap-reverse" => WrapReverse,
                _ => NoWrap,
            };
        }
        _ => {}
    }
}

fn apply_padding(name: &str, value: &str, style: &mut Style) {
    match parse_value(value) {
        Some(UnitSystem::Percent(v)) => match name {
            "padding" => {
                let v = Dimension::Percent(v / 100.0);
                style.padding.top = v;
                style.padding.bottom = v;
                style.padding.start = v;
                style.padding.end = v;
            }
            "padding-bottom" => style.padding.bottom = Dimension::Percent(v / 100.0),
            "padding-left" => style.padding.start = Dimension::Percent(v / 100.0),
            "padding-right" => style.padding.end = Dimension::Percent(v / 100.0),
            "padding-top" => style.padding.top = Dimension::Percent(v / 100.0),
            _ => {}
        },
        Some(UnitSystem::Point(v)) => match name {
            "padding" => {
                style.padding.top = Dimension::Points(v);
                style.padding.bottom = Dimension::Points(v);
                style.padding.start = Dimension::Points(v);
                style.padding.end = Dimension::Points(v);
            }
            "padding-bottom" => style.padding.bottom = Dimension::Points(v),
            "padding-left" => style.padding.start = Dimension::Points(v),
            "padding-right" => style.padding.end = Dimension::Points(v),
            "padding-top" => style.padding.top = Dimension::Points(v),
            _ => {}
        },
        None => {}
    }
}

fn apply_transform(_name: &str, _value: &str, _style: &mut Style) {
    todo!()
}

fn apply_transition(_name: &str, _value: &str, _style: &mut Style) {
    todo!()
}

fn apply_align(name: &str, value: &str, style: &mut Style) {
    match name {
        "align-items" => {
            use AlignItems::*;
            style.align_items = match value {
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
            style.align_content = match value {
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
            style.align_self = match value {
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

pub fn apply_size(_name: &str, _value: &str, _style: &mut Style) {
    //
}

pub fn apply_margin(name: &str, value: &str, style: &mut Style) {
    match parse_value(value) {
        Some(UnitSystem::Percent(v)) => match name {
            "margin" => {
                let v = Dimension::Percent(v / 100.0);
                style.margin.top = v;
                style.margin.bottom = v;
                style.margin.start = v;
                style.margin.end = v;
            }
            "margin-top" => style.margin.top = Dimension::Percent(v / 100.0),
            "margin-bottom" => style.margin.bottom = Dimension::Percent(v / 100.0),
            "margin-left" => style.margin.start = Dimension::Percent(v / 100.0),
            "margin-right" => style.margin.end = Dimension::Percent(v / 100.0),
            _ => {}
        },
        Some(UnitSystem::Point(v)) => match name {
            "margin" => {
                style.margin.top = Dimension::Points(v);
                style.margin.bottom = Dimension::Points(v);
                style.margin.start = Dimension::Points(v);
                style.margin.end = Dimension::Points(v);
            }
            "margin-top" => style.margin.top = Dimension::Points(v),
            "margin-bottom" => style.margin.bottom = Dimension::Points(v),
            "margin-left" => style.margin.start = Dimension::Points(v),
            "margin-right" => style.margin.end = Dimension::Points(v),
            _ => {}
        },
        None => {}
    }
}
