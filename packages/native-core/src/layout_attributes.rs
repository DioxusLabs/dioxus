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

/// applies the entire html namespace defined in dioxus-html
pub fn apply_layout_attributes(name: &str, value: &str, style: &mut Style) {
    if let Ok(property) =
        Property::parse_string(PropertyId::from(name), value, ParserOptions::default())
    {
        match property {
            Property::Display(display) => match display {
                lightningcss::properties::display::Display::Keyword(_) => todo!(),
                lightningcss::properties::display::Display::Pair(pair) => {
                    match pair.outside {
                        lightningcss::properties::display::DisplayOutside::Block => {
                            style.display = Display::None
                        }
                        lightningcss::properties::display::DisplayOutside::Inline => todo!(),
                        lightningcss::properties::display::DisplayOutside::RunIn => todo!(),
                    }
                    match pair.inside {
                        lightningcss::properties::display::DisplayInside::Flow => todo!(),
                        lightningcss::properties::display::DisplayInside::FlowRoot => todo!(),
                        lightningcss::properties::display::DisplayInside::Table => todo!(),
                        lightningcss::properties::display::DisplayInside::Flex(_) => {
                            style.display = Display::Flex
                        }
                        lightningcss::properties::display::DisplayInside::Box(_) => todo!(),
                        lightningcss::properties::display::DisplayInside::Grid => todo!(),
                        lightningcss::properties::display::DisplayInside::Ruby => todo!(),
                    }
                }
            },
            Property::Position(position) => {
                style.position_type = match position {
                    lightningcss::properties::position::Position::Static => todo!(),
                    lightningcss::properties::position::Position::Relative => {
                        PositionType::Relative
                    }
                    lightningcss::properties::position::Position::Absolute => {
                        PositionType::Absolute
                    }
                    lightningcss::properties::position::Position::Sticky(_) => todo!(),
                    lightningcss::properties::position::Position::Fixed => todo!(),
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
            Property::BorderTopWidth(width) => {
                style.border.top = convert_border_side_width(width);
            }
            Property::BorderBottomWidth(width) => {
                style.border.bottom = convert_border_side_width(width);
            }
            Property::BorderLeftWidth(width) => {
                style.border.left = convert_border_side_width(width);
            }
            Property::BorderRightWidth(width) => {
                style.border.right = convert_border_side_width(width);
            }
            Property::BorderWidth(width) => {
                style.border.top = convert_border_side_width(width.top);
                style.border.bottom = convert_border_side_width(width.bottom);
                style.border.left = convert_border_side_width(width.left);
                style.border.right = convert_border_side_width(width.right);
            }
            Property::Border(border) => {
                let width = convert_border_side_width(border.width);
                style.border.top = width;
                style.border.bottom = width;
                style.border.left = width;
                style.border.right = width;
            }
            Property::BorderTop(top) => {
                style.border.top = convert_border_side_width(top.width);
            }
            Property::BorderBottom(bottom) => {
                style.border.bottom = convert_border_side_width(bottom.width);
            }
            Property::BorderLeft(left) => {
                style.border.left = convert_border_side_width(left.width);
            }
            Property::BorderRight(right) => {
                style.border.right = convert_border_side_width(right.width);
            }
            Property::FlexDirection(flex_direction, _) => {
                use FlexDirection::*;
                style.flex_direction = match flex_direction {
                    lightningcss::properties::flex::FlexDirection::Row => Row,
                    lightningcss::properties::flex::FlexDirection::RowReverse => RowReverse,
                    lightningcss::properties::flex::FlexDirection::Column => Column,
                    lightningcss::properties::flex::FlexDirection::ColumnReverse => ColumnReverse,
                }
            }
            Property::FlexWrap(wrap, _) => {
                use FlexWrap::*;
                style.flex_wrap = match wrap {
                    lightningcss::properties::flex::FlexWrap::NoWrap => NoWrap,
                    lightningcss::properties::flex::FlexWrap::Wrap => Wrap,
                    lightningcss::properties::flex::FlexWrap::WrapReverse => WrapReverse,
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
                    lightningcss::properties::align::AlignContent::Normal => todo!(),
                    lightningcss::properties::align::AlignContent::BaselinePosition(_) => {
                        todo!()
                    }
                    lightningcss::properties::align::AlignContent::ContentDistribution(
                        distribution,
                    ) => match distribution {
                        lightningcss::properties::align::ContentDistribution::SpaceBetween => {
                            SpaceBetween
                        }
                        lightningcss::properties::align::ContentDistribution::SpaceAround => {
                            SpaceAround
                        }
                        lightningcss::properties::align::ContentDistribution::SpaceEvenly => {
                            SpaceEvenly
                        }
                        lightningcss::properties::align::ContentDistribution::Stretch => Stretch,
                    },
                    lightningcss::properties::align::AlignContent::ContentPosition(_, position) => {
                        match position {
                            lightningcss::properties::align::ContentPosition::Center => Center,
                            lightningcss::properties::align::ContentPosition::Start => todo!(),
                            lightningcss::properties::align::ContentPosition::End => todo!(),
                            lightningcss::properties::align::ContentPosition::FlexStart => {
                                FlexStart
                            }
                            lightningcss::properties::align::ContentPosition::FlexEnd => FlexEnd,
                        }
                    }
                };
            }
            Property::JustifyContent(justify, _) => {
                use JustifyContent::*;
                style.justify_content = match justify {
                    lightningcss::properties::align::JustifyContent::Normal => todo!(),
                    lightningcss::properties::align::JustifyContent::ContentDistribution(
                        distribution,
                    ) => match distribution {
                        lightningcss::properties::align::ContentDistribution::SpaceBetween => {
                            SpaceBetween
                        }
                        lightningcss::properties::align::ContentDistribution::SpaceAround => {
                            SpaceAround
                        }
                        lightningcss::properties::align::ContentDistribution::SpaceEvenly => {
                            SpaceEvenly
                        }
                        lightningcss::properties::align::ContentDistribution::Stretch => todo!(),
                    },
                    lightningcss::properties::align::JustifyContent::ContentPosition(
                        _,
                        position,
                    ) => match position {
                        lightningcss::properties::align::ContentPosition::Center => Center,
                        lightningcss::properties::align::ContentPosition::Start => todo!(),
                        lightningcss::properties::align::ContentPosition::End => todo!(),
                        lightningcss::properties::align::ContentPosition::FlexStart => FlexStart,
                        lightningcss::properties::align::ContentPosition::FlexEnd => FlexEnd,
                    },
                    lightningcss::properties::align::JustifyContent::Left(_) => todo!(),
                    lightningcss::properties::align::JustifyContent::Right(_) => todo!(),
                };
            }
            Property::AlignSelf(align, _) => {
                use AlignSelf::*;
                style.align_self = match align {
                    lightningcss::properties::align::AlignSelf::Auto => Auto,
                    lightningcss::properties::align::AlignSelf::Normal => todo!(),
                    lightningcss::properties::align::AlignSelf::Stretch => Stretch,
                    lightningcss::properties::align::AlignSelf::BaselinePosition(_) => Baseline,
                    lightningcss::properties::align::AlignSelf::SelfPosition(
                        _overflow,
                        position,
                    ) => match position {
                        lightningcss::properties::align::SelfPosition::Center => Center,
                        lightningcss::properties::align::SelfPosition::Start => todo!(),
                        lightningcss::properties::align::SelfPosition::End => todo!(),
                        lightningcss::properties::align::SelfPosition::SelfStart => todo!(),
                        lightningcss::properties::align::SelfPosition::SelfEnd => todo!(),
                        lightningcss::properties::align::SelfPosition::FlexStart => FlexStart,
                        lightningcss::properties::align::SelfPosition::FlexEnd => FlexEnd,
                    },
                };
            }
            Property::AlignItems(align, _) => {
                use AlignItems::*;
                style.align_items = match align {
                    lightningcss::properties::align::AlignItems::Normal => todo!(),
                    lightningcss::properties::align::AlignItems::BaselinePosition(_) => Baseline,
                    lightningcss::properties::align::AlignItems::Stretch => Stretch,
                    lightningcss::properties::align::AlignItems::SelfPosition(
                        _overflow,
                        position,
                    ) => match position {
                        lightningcss::properties::align::SelfPosition::Center => Center,
                        lightningcss::properties::align::SelfPosition::Start => todo!(),
                        lightningcss::properties::align::SelfPosition::End => todo!(),
                        lightningcss::properties::align::SelfPosition::SelfStart => todo!(),
                        lightningcss::properties::align::SelfPosition::SelfEnd => todo!(),
                        lightningcss::properties::align::SelfPosition::FlexStart => FlexStart,
                        lightningcss::properties::align::SelfPosition::FlexEnd => FlexEnd,
                    },
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

fn convert_dimention_percentage(
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
            convert_dimention_percentage(percentage)
        }
    }
}

fn convert_border_side_width(border_side_width: BorderSideWidth) -> Dimension {
    match border_side_width {
        BorderSideWidth::Length(Length::Value(value)) => convert_length_value(value),
        BorderSideWidth::Thick => Dimension::Points(5.0),
        BorderSideWidth::Medium => Dimension::Points(3.0),
        BorderSideWidth::Thin => Dimension::Points(1.0),
        _ => todo!(),
    }
}

fn convert_gap_value(gap_value: GapValue) -> Dimension {
    match gap_value {
        GapValue::LengthPercentage(dim) => convert_dimention_percentage(dim),
        GapValue::Normal => Dimension::Auto,
    }
}

fn convert_size(size: lightningcss::properties::size::Size) -> Dimension {
    match size {
        lightningcss::properties::size::Size::Auto => Dimension::Auto,
        lightningcss::properties::size::Size::LengthPercentage(length) => {
            convert_dimention_percentage(length)
        }
        lightningcss::properties::size::Size::MinContent(_) => todo!(),
        lightningcss::properties::size::Size::MaxContent(_) => todo!(),
        lightningcss::properties::size::Size::FitContent(_) => todo!(),
        lightningcss::properties::size::Size::FitContentFunction(_) => todo!(),
        lightningcss::properties::size::Size::Stretch(_) => todo!(),
        lightningcss::properties::size::Size::Contain => todo!(),
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
