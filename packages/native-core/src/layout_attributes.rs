/*
- [ ] pub display: Display,
- [x] pub position_type: PositionType,  --> kinda, taffy doesnt support everything
- [ ] pub direction: Direction,

- [x] pub flex_direction: FlexDirection,
- [x] pub flex_wrap: FlexWrap,
- [x] pub flex_grow: f32,
- [x] pub flex_shrink: f32,
- [x] pub flex_basis: Dimension,

- [x] pub overflow: Overflow, ---> kinda implemented... taffy doesnt have support for directional overflow

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

use taffy::{
    prelude::*,
    style::{FlexDirection, PositionType},
};

/// applies the entire html namespace defined in dioxus-html
pub fn apply_layout_attributes(name: &str, value: &str, style: &mut Style) {
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
        "direction" => {}

        "display" => apply_display(name, value, style),

        "empty-cells" => {}

        "flex"
        | "flex-basis"
        | "flex-direction"
        | "flex-flow"
        | "flex-grow"
        | "flex-shrink"
        | "flex-wrap" => apply_flex(name, value, style),

        "float" => {},

        "font-style"
        | "font-variant"
        | "font-weight"
        | "font-size"
        | "line-height"
        | "font-family" => apply_font(name, value, style),

        "height" => {
            if let Some(v) = parse_value(value){
                style.size.height = v;
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
                style.size.width = v;
            }
        }
        "word-break" => {}
        "word-spacing" => {}
        "word-wrap" => {}
        "z-index" => {}
        _ => {}
    }
}

fn apply_font(name: &str, value: &str, style: &mut Style) {
    match name {
        "font-style" => {}
        "font-variant" => {}
        "font-weight" => {}
        "font-size" => {}
        "line-height" => {
            style.size = Size {
                width: style.size.width,
                height: parse_value(value).unwrap_or(Dimension::Points(12.0)),
            }
        }
        "font-family" => {}
        _ => {}
    }
}

/// parse relative or absolute value
pub fn parse_value(value: &str) -> Option<Dimension> {
    if value.ends_with("px") {
        if let Ok(px) = value.trim_end_matches("px").parse::<f32>() {
            Some(Dimension::Points(px))
        } else {
            None
        }
    } else if value.ends_with('%') {
        if let Ok(pct) = value.trim_end_matches('%').parse::<f32>() {
            Some(Dimension::Percent(pct / 100.0))
        } else {
            None
        }
    } else {
        None
    }
}

fn apply_overflow(_name: &str, _value: &str, _style: &mut Style) {
    // todo: add overflow support to taffy
}

fn apply_display(_name: &str, value: &str, style: &mut Style) {
    style.display = match value {
        "flex" => Display::Flex,
        "block" => Display::None,
        _ => Display::Flex,
    }

    // TODO: there are way more variants
    // taffy needs to be updated to handle them
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
                style.border.bottom = v;
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
            if style.border.left == Dimension::default() {
                let v = Dimension::Points(1.0);
                style.border.left = v;
            }
        }
        "border-left-width" => {
            if let Some(v) = parse_value(value) {
                style.border.left = v;
            }
        }
        "border-radius" => {}
        "border-right" => {}
        "border-right-color" => {}
        "border-right-style" => {
            let v = Dimension::Points(1.0);
            style.border.right = v;
        }
        "border-right-width" => {
            if let Some(v) = parse_value(value) {
                style.border.right = v;
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
            if style.border.left == Dimension::default() {
                let v = Dimension::Points(1.0);
                style.border.left = v;
            }
            if style.border.right == Dimension::default() {
                let v = Dimension::Points(1.0);
                style.border.right = v;
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
                style.border.top = v;
            }
        }
        "border-width" => {
            let values: Vec<_> = value.split(' ').collect();
            if values.len() == 1 {
                if let Some(dim) = parse_value(values[0]) {
                    style.border = Rect {
                        right: dim,
                        left: dim,
                        top: dim,
                        bottom: dim,
                    };
                }
            } else {
                let border_widths = [
                    &mut style.border.top,
                    &mut style.border.bottom,
                    &mut style.border.left,
                    &mut style.border.right,
                ];
                for (v, width) in values.into_iter().zip(border_widths) {
                    if let Some(w) = parse_value(v) {
                        *width = w;
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
                style.flex_basis = v;
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
    if let Some(v) = parse_value(value) {
        match name {
            "padding" => {
                style.padding.top = v;
                style.padding.bottom = v;
                style.padding.left = v;
                style.padding.right = v;
            }
            "padding-bottom" => style.padding.bottom = v,
            "padding-left" => style.padding.left = v,
            "padding-right" => style.padding.right = v,
            "padding-top" => style.padding.top = v,
            _ => {}
        }
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

fn apply_margin(name: &str, value: &str, style: &mut Style) {
    if let Some(dim) = parse_value(value) {
        match name {
            "margin" => {
                style.margin.top = dim;
                style.margin.bottom = dim;
                style.margin.left = dim;
                style.margin.right = dim;
            }
            "margin-top" => style.margin.top = dim,
            "margin-bottom" => style.margin.bottom = dim,
            "margin-left" => style.margin.left = dim,
            "margin-right" => style.margin.right = dim,
            _ => {}
        }
    }
}
