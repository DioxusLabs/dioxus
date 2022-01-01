/*
- [ ] pub display: Display,
- [ ] pub position_type: PositionType,
- [ ] pub direction: Direction,

- [x] pub flex_direction: FlexDirection,
- [x] pub flex_wrap: FlexWrap,
- [x] pub flex_grow: f32,
- [x] pub flex_shrink: f32,
- [x] pub flex_basis: Dimension,

- [ ] pub overflow: Overflow,

- [x] pub align_items: AlignItems,
- [x] pub align_self: AlignSelf,
- [x] pub align_content: AlignContent,

- [ ] pub margin: Rect<Dimension>,
- [ ] pub padding: Rect<Dimension>,

- [x] pub justify_content: JustifyContent,
- [ ] pub position: Rect<Dimension>,
- [ ] pub border: Rect<Dimension>,
- [ ] pub size: Size<Dimension>,

- [ ] pub min_size: Size<Dimension>,
- [ ] pub max_size: Size<Dimension>,
- [ ] pub aspect_ratio: Number,
*/

use stretch2::{prelude::*, style::Style};
use tui::style::Style as TuiStyle;

pub struct StyleModifer {
    pub style: Style,
    pub tui_style: TuiStyle,
}

enum TuiModifier {
    Text,
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
            // text color
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
            if value.ends_with("%") {
                if let Ok(pct) = value.trim_end_matches("%").parse::<f32>() {
                    style.style.size.height = Dimension::Percent(pct / 100.0);
                }
            } else if value.ends_with("px") {
                if let Ok(px) = value.trim_end_matches("px").parse::<f32>() {
                    style.style.size.height = Dimension::Points(px);
                }
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

        "position" => {}
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
            if value.ends_with("%") {
                if let Ok(pct) = value.trim_end_matches("%").parse::<f32>() {
                    style.style.size.width = Dimension::Percent(pct / 100.0);
                }
            } else if value.ends_with("px") {
                if let Ok(px) = value.trim_end_matches("px").parse::<f32>() {
                    style.style.size.width = Dimension::Points(px);
                }
            }
        }
        "word-break" => {}
        "word-spacing" => {}
        "word-wrap" => {}
        "z-index" => {}
        _ => {}
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

fn apply_display(name: &str, value: &str, style: &mut StyleModifer) {
    use stretch2::style::Display;
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
            use tui::style::Color;
            match value {
                "red" => style.tui_style.bg.replace(Color::Red),
                "green" => style.tui_style.bg.replace(Color::Green),
                "blue" => style.tui_style.bg.replace(Color::Blue),
                "yellow" => style.tui_style.bg.replace(Color::Yellow),
                "cyan" => style.tui_style.bg.replace(Color::Cyan),
                "magenta" => style.tui_style.bg.replace(Color::Magenta),
                "white" => style.tui_style.bg.replace(Color::White),
                "black" => style.tui_style.bg.replace(Color::Black),
                _ => None,
            };
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
    match name {
        "border" => {}
        "border-bottom" => {}
        "border-bottom-color" => {}
        "border-bottom-left-radius" => {}
        "border-bottom-right-radius" => {}
        "border-bottom-style" => {}
        "border-bottom-width" => {}
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
        "border-left-style" => {}
        "border-left-width" => {}
        "border-radius" => {}
        "border-right" => {}
        "border-right-color" => {}
        "border-right-style" => {}
        "border-right-width" => {}
        "border-spacing" => {}
        "border-style" => {}
        "border-top" => {}
        "border-top-color" => {}
        "border-top-left-radius" => {}
        "border-top-right-radius" => {}
        "border-top-style" => {}
        "border-top-width" => {}
        "border-width" => {
            if let Ok(px) = value.trim_end_matches("px").parse::<f32>() {
                // tuistyle = px;
            }
        }
        _ => {}
    }
}

fn apply_animation(name: &str, value: &str, style: &mut StyleModifer) {
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

fn apply_column(name: &str, value: &str, style: &mut StyleModifer) {
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
            if value.ends_with("%") {
                if let Ok(pct) = value.trim_end_matches("%").parse::<f32>() {
                    style.style.flex_basis = Dimension::Percent(pct / 100.0);
                }
            } else if value.ends_with("px") {
                if let Ok(px) = value.trim_end_matches("px").parse::<f32>() {
                    style.style.flex_basis = Dimension::Points(px);
                }
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
    todo!()
}

fn apply_padding(name: &str, value: &str, style: &mut StyleModifer) {
    // // left
    // start: stretch::style::Dimension::Points(10f32),

    // // right?
    // end: stretch::style::Dimension::Points(10f32),

    // // top?
    // // top: stretch::style::Dimension::Points(10f32),

    // // bottom?
    // // bottom: stretch::style::Dimension::Points(10f32),

    match name {
        "padding" => {
            if name.ends_with("px") {
                if let Ok(px) = value.trim_end_matches("px").parse::<f32>() {
                    style.style.padding.bottom = Dimension::Points(px);
                    style.style.padding.top = Dimension::Points(px);
                    style.style.padding.start = Dimension::Points(px);
                    style.style.padding.end = Dimension::Points(px);
                }
            } else if name.ends_with("%") {
                if let Ok(pct) = value.trim_end_matches("%").parse::<f32>() {
                    //
                }
            }
        }
        "padding-bottom" => {
            if let Ok(px) = value.trim_end_matches("px").parse::<f32>() {
                style.style.padding.bottom = Dimension::Points(px);
            }
        }
        "padding-left" => {
            if let Ok(px) = value.trim_end_matches("px").parse::<f32>() {
                style.style.padding.start = Dimension::Points(px);
            }
        }
        "padding-right" => {
            if let Ok(px) = value.trim_end_matches("px").parse::<f32>() {
                style.style.padding.end = Dimension::Points(px);
            }
        }
        "padding-top" => {
            if let Ok(px) = value.trim_end_matches("px").parse::<f32>() {
                style.style.padding.top = Dimension::Points(px);
            }
        }
        _ => {}
    }
}

fn apply_text(name: &str, value: &str, style: &mut StyleModifer) {
    todo!()
}

fn apply_transform(name: &str, value: &str, style: &mut StyleModifer) {
    todo!()
}

fn apply_transition(name: &str, value: &str, style: &mut StyleModifer) {
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

pub fn apply_size(name: &str, value: &str, style: &mut StyleModifer) {
    //
}

pub fn apply_margin(name: &str, value: &str, style: &mut StyleModifer) {
    match name {
        "margin" => {}
        "margin-bottom" => {}
        "margin-left" => {}
        "margin-right" => {}
        "margin-top" => {}
        _ => {}
    }
}
