//! Dedicated styling system for Components
//! ---------------------------------------
//!
//! In the future, we'd like to move this out of Dioxus core or build a better, more general abstraction. For now, dedicated
//! styling is more-or-less hardcoded into Dioxus.
//!
//!
//!
//!
//!
//!
//!
//!

use crate::innerlude::{Attribute, NodeFactory};

pub struct StyleBuilder;
pub trait AsAttr<'a> {
    fn to_attr(self, field: &'static str, fac: &NodeFactory<'a>) -> Attribute<'a>;
}
impl<'a> AsAttr<'a> for std::fmt::Arguments<'a> {
    fn to_attr(self, field: &'static str, fac: &NodeFactory<'a>) -> Attribute<'a> {
        fac.attr(field, self, Some("style"))
    }
}

macro_rules! build_styles {
    ($ ($name:ident: $lit:literal,)* ) => {
        impl StyleBuilder {
            $(
                pub fn $name<'a>(f: &NodeFactory<'a>, args: impl AsAttr<'a>) -> Attribute<'a> {
                    args.to_attr($lit, f)
                }
            )*
        }
    };
}

build_styles! {
    background: "background",
    background_attachment: "background-attachment",
    background_color: "background-color",
    background_image: "background-image",
    background_position: "background-position",
    background_repeat: "background-repeat",
    border: "border",
    border_bottom: "border-bottom",
    border_bottom_color: "border-bottom-color",
    border_bottom_style: "border-bottom-style",
    border_bottom_width: "border-bottom-width",
    border_color: "border-color",
    border_left: "border-left",
    border_left_color: "border-left-color",
    border_left_style: "border-left-style",
    border_left_width: "border-left-width",
    border_right: "border-right",
    border_right_color: "border-right-color",
    border_right_style: "border-right-style",
    border_right_width: "border-right-width",
    border_style: "border-style",
    border_top: "border-top",
    border_top_color: "border-top-color",
    border_top_style: "border-top-style",
    border_top_width: "border-top-width",
    border_width: "border-width",
    clear: "clear",
    clip: "clip",
    color: "color",
    cursor: "cursor",
    display: "display",
    filter: "filter",
    css_float: "css-float",
    font: "font",
    font_family: "font-family",
    font_size: "font-size",
    font_variant: "font-variant",
    font_weight: "font-weight",
    height: "height",
    left: "left",
    letter_spacing: "letter-spacing",
    line_height: "line-height",
    list_style: "list-style",
    list_style_image: "list-style-image",
    list_style_position: "list-style-position",
    list_style_type: "list-style-type",
    margin: "margin",
    margin_bottom: "margin-bottom",
    margin_left: "margin-left",
    margin_right: "margin-right",
    margin_top: "margin-top",
    overflow: "overflow",
    padding: "padding",
    padding_bottom: "padding-bottom",
    padding_left: "padding-left",
    padding_right: "padding-right",
    padding_top: "padding-top",
    page_break_after: "page-break-after",
    page_break_before: "page-break-before",
    position: "position",
    stroke_dasharray: "stroke-dasharray",
    stroke_dashoffset: "stroke-dashoffset",
    text_align: "text-align",
    text_decoration: "text-decoration",
    text_indent: "text-indent",
    text_transform: "text-transform",
    top: "top",
    vertical_align: "vertical-align",
    visibility: "visibility",
    width: "width",
    z_index: "z-index",
}

fn example(f: &NodeFactory) {
    let style_list = &[("width", "10"), ("text-decoration", "")];
    let styles = &[StyleBuilder::background(f, format_args!("10"))];
}
