//! Utility functions for applying layout attributes to taffy layout

/*
- [ ] pub display: Display, ----> taffy doesnt support all display types
- [x] pub position_type: PositionType,  --> taffy doesnt support everything
- [x] pub direction: Direction,

- [x] pub flex_direction: FlexDirection,
- [x] pub flex_wrap: FlexWrap,
- [x] pub flex_grow: f32,
- [x] pub flex_shrink: f32,
- [x] pub flex_basis: Dimension,

- [x] pub overflow: Overflow, ---> taffy doesnt have support for directional overflow

- [x] pub align_items: AlignItems,
- [x] pub align_self: AlignSelf,
- [x] pub align_content: AlignContent,

- [x] pub margin: Rect<Dimension>,
- [x] pub padding: Rect<Dimension>,

- [x] pub justify_content: JustifyContent,
- [x] pub position: Rect<Dimension>,
- [x] pub border: Rect<Dimension>,

- [ ] pub size: Size<Dimension>, ----> seems to only be relevant for input?
- [ ] pub min_size: Size<Dimension>,
- [ ] pub max_size: Size<Dimension>,

- [ ] pub aspect_ratio: Number, ----> parsing is done, but taffy doesnt support it
*/

use lightningcss::properties::border::LineStyle;
use lightningcss::properties::{align, display, flex, position, size};
use lightningcss::{
    properties::{align::GapValue, border::BorderSideWidth, Property, PropertyId},
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
    style::{FlexDirection, PositionType},
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
                display::Display::Pair(pair) => {
                    if let display::DisplayInside::Flex(_) = pair.inside {
                        style.display = Display::Flex
                    }
                }
                _ => (),
            },
            Property::Position(position) => {
                style.position_type = match position {
                    position::Position::Relative => PositionType::Relative,
                    position::Position::Absolute => PositionType::Absolute,
                    _ => return,
                }
            }
            Property::Top(top) => style.position.top = convert_length_percentage_or_auto(top),
            Property::Bottom(bottom) => {
                style.position.bottom = convert_length_percentage_or_auto(bottom)
            }
            Property::Left(left) => style.position.left = convert_length_percentage_or_auto(left),
            Property::Right(right) => {
                style.position.right = convert_length_percentage_or_auto(right)
            }
            Property::Inset(inset) => {
                style.position.top = convert_length_percentage_or_auto(inset.top);
                style.position.bottom = convert_length_percentage_or_auto(inset.bottom);
                style.position.left = convert_length_percentage_or_auto(inset.left);
                style.position.right = convert_length_percentage_or_auto(inset.right);
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
                    style.border.top =
                        convert_border_side_width(BorderSideWidth::Medium, &config.border_widths);
                }
            }
            Property::BorderBottomStyle(line_style) => {
                if line_style != LineStyle::None {
                    style.border.bottom =
                        convert_border_side_width(BorderSideWidth::Medium, &config.border_widths);
                }
            }
            Property::BorderLeftStyle(line_style) => {
                if line_style != LineStyle::None {
                    style.border.left =
                        convert_border_side_width(BorderSideWidth::Medium, &config.border_widths);
                }
            }
            Property::BorderRightStyle(line_style) => {
                if line_style != LineStyle::None {
                    style.border.right =
                        convert_border_side_width(BorderSideWidth::Medium, &config.border_widths);
                }
            }
            Property::BorderStyle(styles) => {
                if styles.top != LineStyle::None {
                    style.border.top =
                        convert_border_side_width(BorderSideWidth::Medium, &config.border_widths);
                }
                if styles.bottom != LineStyle::None {
                    style.border.bottom =
                        convert_border_side_width(BorderSideWidth::Medium, &config.border_widths);
                }
                if styles.left != LineStyle::None {
                    style.border.left =
                        convert_border_side_width(BorderSideWidth::Medium, &config.border_widths);
                }
                if styles.right != LineStyle::None {
                    style.border.right =
                        convert_border_side_width(BorderSideWidth::Medium, &config.border_widths);
                }
            }
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
                style.flex_basis = convert_length_percentage_or_auto(basis);
            }
            Property::Flex(flex, _) => {
                style.flex_grow = flex.grow;
                style.flex_shrink = flex.shrink;
                style.flex_basis = convert_length_percentage_or_auto(flex.basis);
            }
            Property::AlignContent(align, _) => {
                use AlignContent::*;
                style.align_content = match align {
                    align::AlignContent::ContentDistribution(distribution) => match distribution {
                        align::ContentDistribution::SpaceBetween => SpaceBetween,
                        align::ContentDistribution::SpaceAround => SpaceAround,
                        align::ContentDistribution::SpaceEvenly => SpaceEvenly,
                        align::ContentDistribution::Stretch => Stretch,
                    },
                    align::AlignContent::ContentPosition {
                        value: position, ..
                    } => match position {
                        align::ContentPosition::Center => Center,
                        align::ContentPosition::Start | align::ContentPosition::FlexStart => {
                            FlexStart
                        }
                        align::ContentPosition::End | align::ContentPosition::FlexEnd => FlexEnd,
                    },
                    _ => return,
                };
            }
            Property::JustifyContent(justify, _) => {
                use JustifyContent::*;
                style.justify_content = match justify {
                    align::JustifyContent::ContentDistribution(distribution) => {
                        match distribution {
                            align::ContentDistribution::SpaceBetween => SpaceBetween,
                            align::ContentDistribution::SpaceAround => SpaceAround,
                            align::ContentDistribution::SpaceEvenly => SpaceEvenly,
                            _ => return,
                        }
                    }
                    align::JustifyContent::ContentPosition {
                        value: position, ..
                    } => match position {
                        align::ContentPosition::Center => Center,
                        // start ignores -reverse flex-direction but there is no way to specify that in Taffy
                        align::ContentPosition::Start | align::ContentPosition::FlexStart => {
                            FlexStart
                        }
                        // end ignores -reverse flex-direction but there is no way to specify that in Taffy
                        align::ContentPosition::End | align::ContentPosition::FlexEnd => FlexEnd,
                    },
                    _ => return,
                };
            }
            Property::AlignSelf(align, _) => {
                use AlignSelf::*;
                style.align_self = match align {
                    align::AlignSelf::Auto => Auto,
                    align::AlignSelf::Stretch => Stretch,
                    align::AlignSelf::BaselinePosition(_) => Baseline,
                    align::AlignSelf::SelfPosition {
                        value: position, ..
                    } => match position {
                        align::SelfPosition::Center => Center,
                        align::SelfPosition::Start
                        | align::SelfPosition::SelfStart
                        | align::SelfPosition::FlexStart => FlexStart,
                        align::SelfPosition::End
                        | align::SelfPosition::SelfEnd
                        | align::SelfPosition::FlexEnd => FlexEnd,
                    },
                    _ => return,
                };
            }
            Property::AlignItems(align, _) => {
                use AlignItems::*;
                style.align_items = match align {
                    align::AlignItems::BaselinePosition(_) => Baseline,
                    align::AlignItems::Stretch => Stretch,
                    align::AlignItems::SelfPosition {
                        value: position, ..
                    } => match position {
                        align::SelfPosition::Center => Center,
                        align::SelfPosition::FlexStart => FlexStart,
                        align::SelfPosition::FlexEnd => FlexEnd,
                        _ => return,
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
                style.padding.top = convert_length_percentage_or_auto(padding);
            }
            Property::PaddingBottom(padding) => {
                style.padding.bottom = convert_length_percentage_or_auto(padding);
            }
            Property::PaddingLeft(padding) => {
                style.padding.left = convert_length_percentage_or_auto(padding);
            }
            Property::PaddingRight(padding) => {
                style.padding.right = convert_length_percentage_or_auto(padding);
            }
            Property::Padding(padding) => {
                style.padding = Rect {
                    top: convert_length_percentage_or_auto(padding.top),
                    bottom: convert_length_percentage_or_auto(padding.bottom),
                    left: convert_length_percentage_or_auto(padding.left),
                    right: convert_length_percentage_or_auto(padding.right),
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

fn convert_length_value(length_value: LengthValue) -> Dimension {
    match length_value {
        LengthValue::Px(value) => Dimension::Points(value),
        _ => todo!(),
    }
}

fn convert_dimension_percentage(
    dimension_percentage: DimensionPercentage<LengthValue>,
) -> Dimension {
    match dimension_percentage {
        DimensionPercentage::Dimension(value) => convert_length_value(value),
        DimensionPercentage::Percentage(percentage) => Dimension::Percent(percentage.0),
        _ => todo!(),
    }
}

fn convert_length_percentage_or_auto(
    length_percentage_or_auto: LengthPercentageOrAuto,
) -> Dimension {
    match length_percentage_or_auto {
        LengthPercentageOrAuto::Auto => Dimension::Auto,
        LengthPercentageOrAuto::LengthPercentage(percentage) => {
            convert_dimension_percentage(percentage)
        }
    }
}

fn convert_border_side_width(
    border_side_width: BorderSideWidth,
    border_width_config: &BorderWidths,
) -> Dimension {
    match border_side_width {
        BorderSideWidth::Length(Length::Value(value)) => convert_length_value(value),
        BorderSideWidth::Thick => Dimension::Points(border_width_config.thick),
        BorderSideWidth::Medium => Dimension::Points(border_width_config.medium),
        BorderSideWidth::Thin => Dimension::Points(border_width_config.thin),
        _ => todo!(),
    }
}

fn convert_gap_value(gap_value: GapValue) -> Dimension {
    match gap_value {
        GapValue::LengthPercentage(dim) => convert_dimension_percentage(dim),
        GapValue::Normal => Dimension::Auto,
    }
}

fn convert_size(size: size::Size) -> Dimension {
    match size {
        size::Size::Auto => Dimension::Auto,
        size::Size::LengthPercentage(length) => convert_dimension_percentage(length),
        _ => todo!(),
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
