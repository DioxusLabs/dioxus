//! Utility functions for applying layout attributes to taffy layout

/*
- [ ] pub display: Display, ----> taffy doesnt support all display types
- [x] pub position: Position,  --> taffy doesnt support everything
- [x] pub direction: Direction,

- [x] pub flex_direction: FlexDirection,
- [x] pub flex_wrap: FlexWrap,
- [x] pub flex_grow: f32,
- [x] pub flex_shrink: f32,
- [x] pub flex_basis: Dimension,

- [x]pub grid_auto_flow: GridAutoFlow,
- [x]pub grid_template_rows: GridTrackVec<TrackSizingFunction>,
- [x]pub grid_template_columns: GridTrackVec<TrackSizingFunction>,
- [x]pub grid_auto_rows: GridTrackVec<NonRepeatedTrackSizingFunction>,
- [x]pub grid_auto_columns: GridTrackVec<NonRepeatedTrackSizingFunction>,
- [x]pub grid_row: Line<GridPlacement>,
- [x]pub grid_column: Line<GridPlacement>,

- [x] pub overflow: Overflow, ---> taffy doesnt have support for directional overflow

- [x] pub align_items: AlignItems,
- [x] pub align_self: AlignSelf,
- [x] pub align_content: AlignContent,

- [x] pub margin: Rect<Dimension>,
- [x] pub padding: Rect<Dimension>,

- [x] pub justify_content: JustifyContent,
- [x] pub inset: Rect<Dimension>,
- [x] pub border: Rect<Dimension>,

- [ ] pub size: Size<Dimension>, ----> seems to only be relevant for input?
- [ ] pub min_size: Size<Dimension>,
- [ ] pub max_size: Size<Dimension>,

- [x] pub aspect_ratio: Number,
*/

use lightningcss::properties::border::LineStyle;
use lightningcss::properties::grid::{TrackBreadth, TrackSizing};
use lightningcss::properties::{align, border, display, flex, grid, position, size};
use lightningcss::values::percentage::Percentage;
use lightningcss::{
    properties::{Property, PropertyId},
    stylesheet::ParserOptions,
    traits::Parse,
    values::{
        length::{Length, LengthPercentageOrAuto, LengthValue},
        percentage::DimensionPercentage,
        ratio::Ratio,
    },
};
use taffy::{
    prelude::*,
    style::{FlexDirection, Position},
};

/// Default values for layout attributes
#[derive(Default)]
pub struct LayoutConfigeration {
    /// the default border widths to use
    pub border_widths: BorderWidths,
}

/// Default border widths
pub struct BorderWidths {
    /// the default border width to use for thin borders
    pub thin: f32,
    /// the default border width to use for medium borders
    pub medium: f32,
    /// the default border width to use for thick borders
    pub thick: f32,
}

impl Default for BorderWidths {
    fn default() -> Self {
        Self {
            thin: 1.0,
            medium: 3.0,
            thick: 5.0,
        }
    }
}

/// applies the entire html namespace defined in dioxus-html
pub fn apply_layout_attributes(name: &str, value: &str, style: &mut Style) {
    apply_layout_attributes_cfg(name, value, style, &LayoutConfigeration::default())
}

/// applies the entire html namespace defined in dioxus-html with the specified configeration
pub fn apply_layout_attributes_cfg(
    name: &str,
    value: &str,
    style: &mut Style,
    config: &LayoutConfigeration,
) {
    if let Ok(property) =
        Property::parse_string(PropertyId::from(name), value, ParserOptions::default())
    {
        match property {
            Property::Display(display) => match display {
                display::Display::Keyword(display::DisplayKeyword::None) => {
                    style.display = Display::None
                }
                display::Display::Pair(pair) => match pair.inside {
                    display::DisplayInside::Flex(_) => {
                        style.display = Display::Flex;
                    }
                    display::DisplayInside::Grid => {
                        style.display = Display::Grid;
                    }
                    _ => {}
                },
                _ => {}
            },
            Property::Position(position) => {
                style.position = match position {
                    position::Position::Relative => Position::Relative,
                    position::Position::Absolute => Position::Absolute,
                    _ => return,
                }
            }
            Property::Top(top) => style.inset.top = convert_length_percentage_or_auto(top),
            Property::Bottom(bottom) => {
                style.inset.bottom = convert_length_percentage_or_auto(bottom)
            }
            Property::Left(left) => style.inset.left = convert_length_percentage_or_auto(left),
            Property::Right(right) => style.inset.right = convert_length_percentage_or_auto(right),
            Property::Inset(inset) => {
                style.inset.top = convert_length_percentage_or_auto(inset.top);
                style.inset.bottom = convert_length_percentage_or_auto(inset.bottom);
                style.inset.left = convert_length_percentage_or_auto(inset.left);
                style.inset.right = convert_length_percentage_or_auto(inset.right);
            }
            Property::BorderTopWidth(width) => {
                style.border.top = convert_border_side_width(width, &config.border_widths);
            }
            Property::BorderBottomWidth(width) => {
                style.border.bottom = convert_border_side_width(width, &config.border_widths);
            }
            Property::BorderLeftWidth(width) => {
                style.border.left = convert_border_side_width(width, &config.border_widths);
            }
            Property::BorderRightWidth(width) => {
                style.border.right = convert_border_side_width(width, &config.border_widths);
            }
            Property::BorderWidth(width) => {
                style.border.top = convert_border_side_width(width.top, &config.border_widths);
                style.border.bottom =
                    convert_border_side_width(width.bottom, &config.border_widths);
                style.border.left = convert_border_side_width(width.left, &config.border_widths);
                style.border.right = convert_border_side_width(width.right, &config.border_widths);
            }
            Property::Border(border) => {
                let width = convert_border_side_width(border.width, &config.border_widths);
                style.border.top = width;
                style.border.bottom = width;
                style.border.left = width;
                style.border.right = width;
            }
            Property::BorderTop(top) => {
                style.border.top = convert_border_side_width(top.width, &config.border_widths);
            }
            Property::BorderBottom(bottom) => {
                style.border.bottom =
                    convert_border_side_width(bottom.width, &config.border_widths);
            }
            Property::BorderLeft(left) => {
                style.border.left = convert_border_side_width(left.width, &config.border_widths);
            }
            Property::BorderRight(right) => {
                style.border.right = convert_border_side_width(right.width, &config.border_widths);
            }
            Property::BorderTopStyle(line_style) => {
                if line_style != LineStyle::None {
                    style.border.top = convert_border_side_width(
                        border::BorderSideWidth::Medium,
                        &config.border_widths,
                    );
                }
            }
            Property::BorderBottomStyle(line_style) => {
                if line_style != LineStyle::None {
                    style.border.bottom = convert_border_side_width(
                        border::BorderSideWidth::Medium,
                        &config.border_widths,
                    );
                }
            }
            Property::BorderLeftStyle(line_style) => {
                if line_style != LineStyle::None {
                    style.border.left = convert_border_side_width(
                        border::BorderSideWidth::Medium,
                        &config.border_widths,
                    );
                }
            }
            Property::BorderRightStyle(line_style) => {
                if line_style != LineStyle::None {
                    style.border.right = convert_border_side_width(
                        border::BorderSideWidth::Medium,
                        &config.border_widths,
                    );
                }
            }
            Property::BorderStyle(styles) => {
                if styles.top != LineStyle::None {
                    style.border.top = convert_border_side_width(
                        border::BorderSideWidth::Medium,
                        &config.border_widths,
                    );
                }
                if styles.bottom != LineStyle::None {
                    style.border.bottom = convert_border_side_width(
                        border::BorderSideWidth::Medium,
                        &config.border_widths,
                    );
                }
                if styles.left != LineStyle::None {
                    style.border.left = convert_border_side_width(
                        border::BorderSideWidth::Medium,
                        &config.border_widths,
                    );
                }
                if styles.right != LineStyle::None {
                    style.border.right = convert_border_side_width(
                        border::BorderSideWidth::Medium,
                        &config.border_widths,
                    );
                }
            }

            // Flexbox properties
            Property::FlexDirection(flex_direction, _) => {
                use FlexDirection::*;
                style.flex_direction = match flex_direction {
                    flex::FlexDirection::Row => Row,
                    flex::FlexDirection::RowReverse => RowReverse,
                    flex::FlexDirection::Column => Column,
                    flex::FlexDirection::ColumnReverse => ColumnReverse,
                }
            }
            Property::FlexWrap(wrap, _) => {
                use FlexWrap::*;
                style.flex_wrap = match wrap {
                    flex::FlexWrap::NoWrap => NoWrap,
                    flex::FlexWrap::Wrap => Wrap,
                    flex::FlexWrap::WrapReverse => WrapReverse,
                }
            }
            Property::FlexGrow(grow, _) => {
                style.flex_grow = grow;
            }
            Property::FlexShrink(shrink, _) => {
                style.flex_shrink = shrink;
            }
            Property::FlexBasis(basis, _) => {
                style.flex_basis = convert_length_percentage_or_auto(basis).into();
            }
            Property::Flex(flex, _) => {
                style.flex_grow = flex.grow;
                style.flex_shrink = flex.shrink;
                style.flex_basis = convert_length_percentage_or_auto(flex.basis).into();
            }

            // Grid properties
            Property::GridAutoFlow(grid_auto_flow) => {
                let is_row = grid_auto_flow.contains(grid::GridAutoFlow::Row);
                let is_dense = grid_auto_flow.contains(grid::GridAutoFlow::Dense);
                style.grid_auto_flow = match (is_row, is_dense) {
                    (true, false) => GridAutoFlow::Row,
                    (false, false) => GridAutoFlow::Column,
                    (true, true) => GridAutoFlow::RowDense,
                    (false, true) => GridAutoFlow::ColumnDense,
                };
            }
            Property::GridTemplateColumns(TrackSizing::TrackList(track_list)) => {
                style.grid_template_columns = track_list
                    .items
                    .into_iter()
                    .map(convert_grid_track_item)
                    .collect();
            }
            Property::GridTemplateRows(TrackSizing::TrackList(track_list)) => {
                style.grid_template_rows = track_list
                    .items
                    .into_iter()
                    .map(convert_grid_track_item)
                    .collect();
            }
            Property::GridAutoColumns(grid::TrackSizeList(track_size_list)) => {
                style.grid_auto_columns = track_size_list
                    .into_iter()
                    .map(convert_grid_track_size)
                    .collect();
            }
            Property::GridAutoRows(grid::TrackSizeList(track_size_list)) => {
                style.grid_auto_rows = track_size_list
                    .into_iter()
                    .map(convert_grid_track_size)
                    .collect();
            }
            Property::GridRow(grid_row) => {
                style.grid_row = Line {
                    start: convert_grid_placement(grid_row.start),
                    end: convert_grid_placement(grid_row.end),
                };
            }
            Property::GridColumn(grid_column) => {
                style.grid_column = Line {
                    start: convert_grid_placement(grid_column.start),
                    end: convert_grid_placement(grid_column.end),
                };
            }

            // Alignment properties
            Property::AlignContent(align, _) => {
                use AlignContent::*;
                style.align_content = match align {
                    align::AlignContent::ContentDistribution(distribution) => match distribution {
                        align::ContentDistribution::SpaceBetween => Some(SpaceBetween),
                        align::ContentDistribution::SpaceAround => Some(SpaceAround),
                        align::ContentDistribution::SpaceEvenly => Some(SpaceEvenly),
                        align::ContentDistribution::Stretch => Some(Stretch),
                    },
                    align::AlignContent::ContentPosition {
                        value: position, ..
                    } => match position {
                        align::ContentPosition::Center => Some(Center),
                        align::ContentPosition::Start => Some(Start),
                        align::ContentPosition::FlexStart => Some(FlexStart),
                        align::ContentPosition::End => Some(End),
                        align::ContentPosition::FlexEnd => Some(FlexEnd),
                    },
                    _ => return,
                };
            }
            Property::JustifyContent(justify, _) => {
                use AlignContent::*;
                style.justify_content = match justify {
                    align::JustifyContent::ContentDistribution(distribution) => {
                        match distribution {
                            align::ContentDistribution::SpaceBetween => Some(SpaceBetween),
                            align::ContentDistribution::SpaceAround => Some(SpaceAround),
                            align::ContentDistribution::SpaceEvenly => Some(SpaceEvenly),
                            _ => return,
                        }
                    }
                    align::JustifyContent::ContentPosition {
                        value: position, ..
                    } => match position {
                        align::ContentPosition::Center => Some(Center),
                        align::ContentPosition::Start => Some(Start),
                        align::ContentPosition::FlexStart => Some(FlexStart),
                        align::ContentPosition::End => Some(End),
                        align::ContentPosition::FlexEnd => Some(FlexEnd),
                    },
                    _ => return,
                };
            }
            Property::AlignSelf(align, _) => {
                use AlignItems::*;
                style.align_self = match align {
                    align::AlignSelf::Auto => None,
                    align::AlignSelf::Stretch => Some(Stretch),
                    align::AlignSelf::BaselinePosition(_) => Some(Baseline),
                    align::AlignSelf::SelfPosition {
                        value: position, ..
                    } => match position {
                        align::SelfPosition::Center => Some(Center),
                        align::SelfPosition::Start | align::SelfPosition::SelfStart => Some(Start),
                        align::SelfPosition::FlexStart => Some(FlexStart),
                        align::SelfPosition::End | align::SelfPosition::SelfEnd => Some(End),
                        align::SelfPosition::FlexEnd => Some(FlexEnd),
                    },
                    _ => return,
                };
            }
            Property::AlignItems(align, _) => {
                use AlignItems::*;
                style.align_items = match align {
                    align::AlignItems::BaselinePosition(_) => Some(Baseline),
                    align::AlignItems::Stretch => Some(Stretch),
                    align::AlignItems::SelfPosition {
                        value: position, ..
                    } => match position {
                        align::SelfPosition::Center => Some(Center),
                        align::SelfPosition::FlexStart => Some(FlexStart),
                        align::SelfPosition::FlexEnd => Some(FlexEnd),
                        align::SelfPosition::Start | align::SelfPosition::SelfStart => {
                            Some(FlexEnd)
                        }
                        align::SelfPosition::End | align::SelfPosition::SelfEnd => Some(FlexEnd),
                    },
                    _ => return,
                };
            }
            Property::RowGap(row_gap) => {
                style.gap.width = convert_gap_value(row_gap);
            }
            Property::ColumnGap(column_gap) => {
                style.gap.height = convert_gap_value(column_gap);
            }
            Property::Gap(gap) => {
                style.gap = Size {
                    width: convert_gap_value(gap.row),
                    height: convert_gap_value(gap.column),
                };
            }
            Property::MarginTop(margin) => {
                style.margin.top = convert_length_percentage_or_auto(margin);
            }
            Property::MarginBottom(margin) => {
                style.margin.bottom = convert_length_percentage_or_auto(margin);
            }
            Property::MarginLeft(margin) => {
                style.margin.left = convert_length_percentage_or_auto(margin);
            }
            Property::MarginRight(margin) => {
                style.margin.right = convert_length_percentage_or_auto(margin);
            }
            Property::Margin(margin) => {
                style.margin = Rect {
                    top: convert_length_percentage_or_auto(margin.top),
                    bottom: convert_length_percentage_or_auto(margin.bottom),
                    left: convert_length_percentage_or_auto(margin.left),
                    right: convert_length_percentage_or_auto(margin.right),
                };
            }
            Property::PaddingTop(padding) => {
                style.padding.top = convert_padding(padding);
            }
            Property::PaddingBottom(padding) => {
                style.padding.bottom = convert_padding(padding);
            }
            Property::PaddingLeft(padding) => {
                style.padding.left = convert_padding(padding);
            }
            Property::PaddingRight(padding) => {
                style.padding.right = convert_padding(padding);
            }
            Property::Padding(padding) => {
                style.padding = Rect {
                    top: convert_padding(padding.top),
                    bottom: convert_padding(padding.bottom),
                    left: convert_padding(padding.left),
                    right: convert_padding(padding.right),
                };
            }
            Property::Width(width) => {
                style.size.width = convert_size(width);
            }
            Property::Height(height) => {
                style.size.height = convert_size(height);
            }
            _ => (),
        }
        // currently not implemented in lightningcss
        if name == "aspect-ratio" {
            if let Ok(ratio) = Ratio::parse_string(value) {
                style.aspect_ratio = Some(ratio.0 / ratio.1);
            }
        }
    }
}

fn extract_px_value(length_value: LengthValue) -> f32 {
    match length_value {
        LengthValue::Px(value) => value,
        _ => todo!("Only px values are supported"),
    }
}

fn convert_length_percentage(
    dimension_percentage: DimensionPercentage<LengthValue>,
) -> LengthPercentage {
    match dimension_percentage {
        DimensionPercentage::Dimension(value) => LengthPercentage::Points(extract_px_value(value)),
        DimensionPercentage::Percentage(percentage) => LengthPercentage::Percent(percentage.0),
        DimensionPercentage::Calc(_) => todo!("Calc is not supported yet"),
    }
}

fn convert_padding(dimension_percentage: LengthPercentageOrAuto) -> LengthPercentage {
    match dimension_percentage {
        LengthPercentageOrAuto::Auto => unimplemented!(),
        LengthPercentageOrAuto::LengthPercentage(lp) => match lp {
            DimensionPercentage::Dimension(value) => {
                LengthPercentage::Points(extract_px_value(value))
            }
            DimensionPercentage::Percentage(percentage) => LengthPercentage::Percent(percentage.0),
            DimensionPercentage::Calc(_) => unimplemented!("Calc is not supported yet"),
        },
    }
}

fn convert_length_percentage_or_auto(
    dimension_percentage: LengthPercentageOrAuto,
) -> LengthPercentageAuto {
    match dimension_percentage {
        LengthPercentageOrAuto::Auto => LengthPercentageAuto::Auto,
        LengthPercentageOrAuto::LengthPercentage(lp) => match lp {
            DimensionPercentage::Dimension(value) => {
                LengthPercentageAuto::Points(extract_px_value(value))
            }
            DimensionPercentage::Percentage(percentage) => {
                LengthPercentageAuto::Percent(percentage.0)
            }
            DimensionPercentage::Calc(_) => todo!("Calc is not supported yet"),
        },
    }
}

fn convert_dimension(dimension_percentage: DimensionPercentage<LengthValue>) -> Dimension {
    match dimension_percentage {
        DimensionPercentage::Dimension(value) => Dimension::Points(extract_px_value(value)),
        DimensionPercentage::Percentage(percentage) => Dimension::Percent(percentage.0),
        DimensionPercentage::Calc(_) => todo!("Calc is not supported yet"),
    }
}

fn convert_border_side_width(
    border_side_width: border::BorderSideWidth,
    border_width_config: &BorderWidths,
) -> LengthPercentage {
    match border_side_width {
        border::BorderSideWidth::Length(Length::Value(value)) => {
            LengthPercentage::Points(extract_px_value(value))
        }
        border::BorderSideWidth::Thick => LengthPercentage::Points(border_width_config.thick),
        border::BorderSideWidth::Medium => LengthPercentage::Points(border_width_config.medium),
        border::BorderSideWidth::Thin => LengthPercentage::Points(border_width_config.thin),
        border::BorderSideWidth::Length(_) => todo!("Only Length::Value is supported"),
    }
}

fn convert_gap_value(gap_value: align::GapValue) -> LengthPercentage {
    match gap_value {
        align::GapValue::LengthPercentage(dim) => convert_length_percentage(dim),
        align::GapValue::Normal => LengthPercentage::Points(0.0),
    }
}

fn convert_size(size: size::Size) -> Dimension {
    match size {
        size::Size::Auto => Dimension::Auto,
        size::Size::LengthPercentage(length) => convert_dimension(length),
        size::Size::MinContent(_) => Dimension::Auto, // Unimplemented, so default auto
        size::Size::MaxContent(_) => Dimension::Auto, // Unimplemented, so default auto
        size::Size::FitContent(_) => Dimension::Auto, // Unimplemented, so default auto
        size::Size::FitContentFunction(_) => Dimension::Auto, // Unimplemented, so default auto
        size::Size::Stretch(_) => Dimension::Auto,    // Unimplemented, so default auto
        size::Size::Contain => Dimension::Auto,       // Unimplemented, so default auto
    }
}

fn convert_grid_placement(input: grid::GridLine) -> GridPlacement {
    match input {
        grid::GridLine::Auto => GridPlacement::Auto,
        grid::GridLine::Line { index, .. } => line(index as i16),
        grid::GridLine::Span { index, .. } => span(index as u16),
        grid::GridLine::Area { .. } => unimplemented!(),
    }
}

fn convert_grid_track_item(input: grid::TrackListItem) -> TrackSizingFunction {
    match input {
        grid::TrackListItem::TrackSize(size) => {
            TrackSizingFunction::Single(convert_grid_track_size(size))
        }
        grid::TrackListItem::TrackRepeat(_) => todo!("requires TrackRepeat fields to be public!"),
    }
}

fn convert_grid_track_size(input: grid::TrackSize) -> NonRepeatedTrackSizingFunction {
    match input {
        grid::TrackSize::TrackBreadth(breadth) => minmax(
            convert_track_breadth_min(&breadth),
            convert_track_breadth_max(&breadth),
        ),
        grid::TrackSize::MinMax { min, max } => minmax(
            convert_track_breadth_min(&min),
            convert_track_breadth_max(&max),
        ),
        grid::TrackSize::FitContent(limit) => match limit {
            DimensionPercentage::Dimension(LengthValue::Px(len)) => minmax(auto(), points(len)),
            DimensionPercentage::Percentage(Percentage(pct)) => minmax(auto(), percent(pct)),
            _ => unimplemented!(),
        },
    }
}

fn convert_track_breadth_max(breadth: &TrackBreadth) -> MaxTrackSizingFunction {
    match breadth {
        grid::TrackBreadth::Length(length_percentage) => match length_percentage {
            DimensionPercentage::Dimension(LengthValue::Px(len)) => points(*len),
            DimensionPercentage::Percentage(Percentage(pct)) => percent(*pct),
            _ => unimplemented!(),
        },
        grid::TrackBreadth::Flex(fraction) => fr(*fraction),
        grid::TrackBreadth::MinContent => MaxTrackSizingFunction::MinContent,
        grid::TrackBreadth::MaxContent => MaxTrackSizingFunction::MaxContent,
        grid::TrackBreadth::Auto => MaxTrackSizingFunction::Auto,
    }
}

fn convert_track_breadth_min(breadth: &TrackBreadth) -> MinTrackSizingFunction {
    match breadth {
        grid::TrackBreadth::Length(length_percentage) => match length_percentage {
            DimensionPercentage::Dimension(LengthValue::Px(len)) => points(*len),
            DimensionPercentage::Percentage(Percentage(pct)) => percent(*pct),
            _ => unimplemented!(),
        },
        grid::TrackBreadth::MinContent => MinTrackSizingFunction::MinContent,
        grid::TrackBreadth::MaxContent => MinTrackSizingFunction::MaxContent,
        grid::TrackBreadth::Auto => MinTrackSizingFunction::Auto,
        grid::TrackBreadth::Flex(_) => MinTrackSizingFunction::Auto,
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
