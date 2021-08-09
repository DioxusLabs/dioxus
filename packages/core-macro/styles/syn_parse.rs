//! Implementation of `syn::parse::Parse` for styles, and associated helper data/functions.
// TODO make all parsers use HyphenWord where appropriate.
// TODO make all error messages nice
// TODO 100% test coverage
// TODO see if I can get https://github.com/rust-lang/rust/issues/67544 accepted. then change "em" to
// em and "ex" to ex.
// TODO Split out extra "Dynamic" layer for each type for use in proc macro (so we can have `{ <arbitary
// rust code> }`)
use crate::*;
use proc_macro2::Span;
use std::{
    cell::RefCell,
    collections::BTreeSet,
    fmt::{self, Write},
    ops::RangeBounds,
    str,
};
use syn::{
    ext::IdentExt,
    parse::{discouraged::Speculative, Parse, ParseStream},
    punctuated::Punctuated,
    spanned::Spanned,
    Ident, Token,
};

use super::{DynamicStyle, DynamicStyles, Styles};

impl Parse for DynamicStyles {
    fn parse(s: ParseStream) -> syn::Result<Self> {
        let punc = s.parse_terminated::<_, Token![;]>(<DynamicStyle as Parse>::parse)?;
        Ok(DynamicStyles::from(punc.into_iter().collect::<Vec<_>>()))
    }
}

impl Parse for Styles {
    fn parse(s: ParseStream) -> syn::Result<Self> {
        let punc = s.parse_terminated::<_, Token![;]>(<Style as Parse>::parse)?;
        Ok(Styles::from(punc.into_iter().collect::<Vec<_>>()))
    }
}

impl Parse for DynamicStyle {
    fn parse(s: ParseStream) -> syn::Result<Self> {
        // Pass through brackets
        if s.peek(syn::token::Brace) {
            Ok(DynamicStyle::Dynamic(s.parse()?))
        } else {
            Ok(DynamicStyle::Literal(s.parse()?))
        }
    }
}

impl Parse for Style {
    fn parse(s: ParseStream) -> syn::Result<Self> {
        if s.peek(syn::LitStr) {
            let unchecked: syn::LitStr = s.parse()?;
            return Ok(Style::Unchecked(unchecked.value()));
        }

        let name: HyphenWord = s.parse()?;
        if name.try_match("dummy") {
            return Ok(Style::Dummy);
        }

        s.parse::<Token![:]>()?;

        let output = if name.try_match("align-content") {
            Style::AlignContent(s.parse()?)
        } else if name.try_match("align-items") {
            Style::AlignItems(s.parse()?)
        } else if name.try_match("align-self") {
            Style::AlignSelf(s.parse()?)
        // all
        // background
        } else if name.try_match("background-attachment") {
            Style::BackgroundAttachment(s.parse()?)
        } else if name.try_match("background-blend-mode") {
            Style::BackgroundBlendMode(s.parse()?)
        } else if name.try_match("background-clip") {
            Style::BackgroundClip(s.parse()?)
        } else if name.try_match("background-color") {
            Style::BackgroundColor(s.parse()?)
        } else if name.try_match("background-image") {
            Style::BackgroundImage(s.parse()?)
        } else if name.try_match("background-origin") {
            Style::BackgroundOrigin(s.parse()?)
        } else if name.try_match("background-position") {
            Style::BackgroundPosition(s.parse()?)
        } else if name.try_match("background-repeat") {
            Style::BackgroundRepeat(s.parse()?)
        } else if name.try_match("background-size") {
            Style::BackgroundSize(s.parse()?)
        } else if name.try_match("border") {
            Style::Border(s.parse()?)
        } else if name.try_match("border-bottom") {
            Style::BorderBottom(s.parse()?)
        } else if name.try_match("border-bottom-color") {
            Style::BorderBottomColor(s.parse()?)
        } else if name.try_match("border-bottom-left-radius") {
            Style::BorderBottomLeftRadius(s.parse()?)
        } else if name.try_match("border-bottom-right-radius") {
            Style::BorderBottomRightRadius(s.parse()?)
        } else if name.try_match("border-bottom-style") {
            Style::BorderBottomStyle(s.parse()?)
        } else if name.try_match("border-bottom-width") {
            Style::BorderBottomWidth(s.parse()?)
        } else if name.try_match("border-collapse") {
            Style::BorderCollapse(s.parse()?)
        } else if name.try_match("border-color") {
            Style::BorderColor(s.parse()?)
        // border-image
        // border-image-outset
        // border-image-repeat
        // border-image-slice
        // border-image-source
        // border-image-width
        } else if name.try_match("border-left") {
            Style::BorderLeft(s.parse()?)
        } else if name.try_match("border-left-color") {
            Style::BorderLeftColor(s.parse()?)
        } else if name.try_match("border-left-style") {
            Style::BorderLeftStyle(s.parse()?)
        } else if name.try_match("border-left-width") {
            Style::BorderLeftWidth(s.parse()?)
        } else if name.try_match("border-radius") {
            Style::BorderRadius(s.parse()?)
        } else if name.try_match("border-right") {
            Style::BorderRight(s.parse()?)
        } else if name.try_match("border-right-color") {
            Style::BorderRightColor(s.parse()?)
        } else if name.try_match("border-right-style") {
            Style::BorderRightStyle(s.parse()?)
        } else if name.try_match("border-right-width") {
            Style::BorderRightWidth(s.parse()?)
        // border-spacing
        } else if name.try_match("border-style") {
            Style::BorderStyle(s.parse()?)
        } else if name.try_match("border-top") {
            Style::BorderTop(s.parse()?)
        } else if name.try_match("border-top-color") {
            Style::BorderTopColor(s.parse()?)
        } else if name.try_match("border-top-left-radius") {
            Style::BorderTopLeftRadius(s.parse()?)
        } else if name.try_match("border-top-right-radius") {
            Style::BorderTopRightRadius(s.parse()?)
        } else if name.try_match("border-top-style") {
            Style::BorderTopStyle(s.parse()?)
        } else if name.try_match("border-top-width") {
            Style::BorderTopWidth(s.parse()?)
        } else if name.try_match("border-width") {
            Style::BorderWidth(s.parse()?)
        } else if name.try_match("bottom") {
            Style::Bottom(s.parse()?)
        // box-decoration-break
        } else if name.try_match("box-shadow") {
            Style::BoxShadow(s.parse()?)
        } else if name.try_match("box-sizing") {
            Style::BoxSizing(s.parse()?)
        // break-after
        // break-before
        // break-inside
        // caption-side
        // caret-color
        } else if name.try_match("clear") {
            Style::Clear(s.parse()?)
        // clip
        // clip-path
        // clip-rule
        } else if name.try_match("column-count") {
            Style::ColumnCount(s.parse()?)
        } else if name.try_match("color") {
            Style::Color(s.parse()?)
        // contain
        // content
        // counter-increment
        // counter-reset
        // cue
        // cue-after
        // cue-before
        } else if name.try_match("cursor") {
            Style::Cursor(s.parse()?)
        // direction
        } else if name.try_match("display") {
            Style::Display(s.parse()?)
        // elevation
        // empty-cells
        // flex
        } else if name.try_match("flex-basis") {
            Style::FlexBasis(s.parse()?)
        } else if name.try_match("flex-direction") {
            Style::FlexDirection(s.parse()?)
        // flex-flow
        } else if name.try_match("flex-grow") {
            let number: Number = s.parse()?;
            if !number.suffix.is_empty() {
                return Err(syn::Error::new(number.span, "expected number"));
            }
            Style::FlexGrow(number.value)
        } else if name.try_match("flex-shrink") {
            let number: Number = s.parse()?;
            if !number.suffix.is_empty() {
                return Err(syn::Error::new(number.span, "expected number"));
            }
            Style::FlexShrink(number.value)
        } else if name.try_match("flex-wrap") {
            Style::FlexWrap(s.parse()?)
        } else if name.try_match("float") {
            Style::Float(s.parse()?)
        // font
        } else if name.try_match("font-family") {
            Style::FontFamily(s.parse()?)
        // font-feature-settings
        // font-kerning
        } else if name.try_match("font-size") {
            Style::FontSize(s.parse()?)
        // font-size-adjust
        // font-stretch
        } else if name.try_match("font-style") {
            Style::FontStyle(s.parse()?)
        // font-synthesis
        // font-variant
        // font-variant-caps
        // font-variant-east-asian
        // font-variant-ligatures
        // font-variant-numeric
        // font-variant-position
        } else if name.try_match("font-weight") {
            Style::FontWeight(s.parse()?)
        // glyph-orientation-vertical
        // grid
        // grid-area
        // grid-auto-columns
        // grid-auto-flow
        // grid-auto-rows
        // grid-column
        // grid-column-end
        // grid-column-start
        // grid-row
        // grid-row-end
        // grid-row-start
        // grid-template
        // grid-template-areas
        // grid-template-columns
        // grid-template-rows
        } else if name.try_match("height") {
            Style::Height(s.parse()?)
        // image-orientation
        // image-rendering
        // isolation
        } else if name.try_match("justify-content") {
            Style::JustifyContent(s.parse()?)
        } else if name.try_match("left") {
            Style::Left(s.parse()?)
        // letter-spacing
        } else if name.try_match("line-height") {
            Style::LineHeight(s.parse()?)
        // list-style
        // list-style-image
        // list-style-position
        } else if name.try_match("list-style-type") {
            Style::ListStyleType(s.parse()?)
        } else if name.try_match("margin") {
            Style::Margin(s.parse()?)
        } else if name.try_match("margin-bottom") {
            Style::MarginBottom(s.parse()?)
        } else if name.try_match("margin-left") {
            Style::MarginLeft(s.parse()?)
        } else if name.try_match("margin-right") {
            Style::MarginRight(s.parse()?)
        } else if name.try_match("margin-top") {
            Style::MarginTop(s.parse()?)
        // mask
        // mask-border
        // mask-border-mode
        // mask-border-outset
        // mask-border-repeat
        // mask-border-slice
        // mask-border-source
        // mask-border-width
        // mask-clip
        // mask-composite
        // mask-image
        // mask-mode
        // mask-origin
        // mask-position
        // mask-repeat
        // mask-size
        // mask-type
        } else if name.try_match("max-height") {
            Style::MaxHeight(s.parse()?)
        } else if name.try_match("max-width") {
            Style::MaxWidth(s.parse()?)
        } else if name.try_match("min-height") {
            Style::MinHeight(s.parse()?)
        } else if name.try_match("min-width") {
            Style::MinWidth(s.parse()?)
        // mix-blend-mode
        } else if name.try_match("object-fit") {
            Style::ObjectFit(s.parse()?)
        // object-position
        // opacity
        // order
        // orphans
        // outline
        // outline-color
        // outline-offset
        // outline-style
        // outline-width
        } else if name.try_match("overflow") {
            Style::Overflow(s.parse()?)
        } else if name.try_match("overflow-x") {
            Style::OverflowX(s.parse()?)
        } else if name.try_match("overflow-y") {
            Style::OverflowY(s.parse()?)
        } else if name.try_match("padding") {
            Style::Padding(s.parse()?)
        } else if name.try_match("padding-bottom") {
            Style::PaddingBottom(s.parse()?)
        } else if name.try_match("padding-left") {
            Style::PaddingLeft(s.parse()?)
        } else if name.try_match("padding-right") {
            Style::PaddingRight(s.parse()?)
        } else if name.try_match("padding-top") {
            Style::PaddingTop(s.parse()?)
        // page-break-after
        // page-break-before
        // page-break-inside
        // pause
        // pause-after
        // pause-before
        // pitch
        // pitch-range
        // play-during
        } else if name.try_match("position") {
            Style::Position(s.parse()?)
        // quotes
        } else if name.try_match("resize") {
            Style::Resize(s.parse()?)
        // richness
        } else if name.try_match("right") {
            Style::Right(s.parse()?)
        // scroll-margin
        // scroll-margin-block
        // scroll-margin-block-end
        // scroll-margin-block-start
        // scroll-margin-bottom
        // scroll-margin-inline
        // scroll-margin-inline-end
        // scroll-margin-inline-start
        // scroll-margin-left
        // scroll-margin-right
        // scroll-margin-top
        // scroll-padding
        // scroll-padding-block
        // scroll-padding-block-end
        // scroll-padding-block-start
        // scroll-padding-bottom
        // scroll-padding-inline
        // scroll-padding-inline-end
        // scroll-padding-inline-start
        // scroll-padding-left
        // scroll-padding-right
        // scroll-padding-top
        // scroll-snap-align
        // scroll-snap-stop
        // scroll-snap-type
        // shape-image-threshold
        // shape-margin
        // shape-outside
        // speak
        // speak-header
        // speak-numeral
        // speak-punctuation
        // speech-rate
        // stress
        // table-layout
        } else if name.try_match("text-align") {
            Style::TextAlign(s.parse()?)
        // text-combine-upright
        // text-decoration
        // text-decoration-color
        // text-decoration-line
        // text-decoration-style
        // text-emphasis
        // text-emphasis-color
        // text-emphasis-position
        // text-emphasis-style
        // text-indent
        // text-orientation
        // text-overflow
        // text-shadow
        // text-transform
        // text-underline-position
        } else if name.try_match("top") {
            Style::Top(s.parse()?)
        // transform
        // transform-box
        // transform-origin
        // unicode-bidi
        // vertical-align
        // visibility
        // voice-family
        // volume
        } else if name.try_match("white-space") {
            Style::WhiteSpace(s.parse()?)
        } else if name.try_match("widows") {
            Style::Widows(integer(s, 1..)?)
        } else if name.try_match("width") {
            Style::Width(s.parse()?)
        // will-change
        // word-spacing
        // writing-mode
        // z-index
        } else {
            return Err(name.error());
        };

        if !finished_rule(s) {
            return Err(s.error("unexpected trailing tokens in style rule"));
        }

        Ok(output)
    }
}

impl Parse for AlignContent {
    fn parse(s: ParseStream) -> syn::Result<Self> {
        let name: HyphenWord = s.parse()?;

        if name.try_match("flex-start") {
            Ok(AlignContent::FlexStart)
        } else if name.try_match("flex-end") {
            Ok(AlignContent::FlexEnd)
        } else if name.try_match("center") {
            Ok(AlignContent::Center)
        } else if name.try_match("space-between") {
            Ok(AlignContent::SpaceBetween)
        } else if name.try_match("space-around") {
            Ok(AlignContent::SpaceAround)
        } else if name.try_match("stretch") {
            Ok(AlignContent::Stretch)
        } else {
            Err(name.error())
        }
    }
}

#[test]
fn test_align_content() {
    for test in vec![
        "flex-start",
        "flex-end",
        "center",
        "space-between",
        "space-around",
        "stretch",
    ] {
        assert_eq!(
            &syn::parse_str::<AlignContent>(test).unwrap().to_string(),
            test
        );
    }
    assert_eq!(
        &syn::parse_str::<Style>("align-content:flex-start")
            .unwrap()
            .to_string(),
        "align-content:flex-start"
    );
}

impl Parse for AlignItems {
    fn parse(s: ParseStream) -> syn::Result<Self> {
        let word: HyphenWord = s.parse()?;
        if word.try_match("normal") {
            Ok(AlignItems::Normal)
        } else if word.try_match("stretch") {
            Ok(AlignItems::Stretch)
        } else if word.try_match("center") {
            Ok(AlignItems::Center)
        } else if word.try_match("start") {
            Ok(AlignItems::Start)
        } else if word.try_match("end") {
            Ok(AlignItems::End)
        } else if word.try_match("flex-start") {
            Ok(AlignItems::FlexStart)
        } else if word.try_match("flex-end") {
            Ok(AlignItems::FlexEnd)
        } else if word.try_match("baseline") {
            Ok(AlignItems::Baseline)
        } else if word.try_match("first") {
            let word: HyphenWord = s.parse()?;
            if word.try_match("baseline") {
                Ok(AlignItems::FirstBaseline)
            } else {
                Err(word.error())
            }
        } else if word.try_match("last") {
            let word: HyphenWord = s.parse()?;
            if word.try_match("baseline") {
                Ok(AlignItems::LastBaseline)
            } else {
                Err(word.error())
            }
        } else if word.try_match("safe") {
            let word: HyphenWord = s.parse()?;
            if word.try_match("center") {
                Ok(AlignItems::SafeCenter)
            } else {
                Err(word.error())
            }
        } else if word.try_match("unsafe") {
            let word: HyphenWord = s.parse()?;
            if word.try_match("center") {
                Ok(AlignItems::UnsafeCenter)
            } else {
                Err(word.error())
            }
        } else {
            Err(word.error())
        }
    }
}

impl Parse for AlignSelf {
    fn parse(s: ParseStream) -> syn::Result<Self> {
        let word: HyphenWord = s.parse()?;
        if word.try_match("auto") {
            Ok(AlignSelf::Auto)
        } else if word.try_match("normal") {
            Ok(AlignSelf::Normal)
        } else if word.try_match("center") {
            Ok(AlignSelf::Center)
        } else if word.try_match("start") {
            Ok(AlignSelf::Start)
        } else if word.try_match("self-start") {
            Ok(AlignSelf::SelfStart)
        } else if word.try_match("self-end") {
            Ok(AlignSelf::SelfEnd)
        } else if word.try_match("flex-start") {
            Ok(AlignSelf::FlexStart)
        } else if word.try_match("flex-end") {
            Ok(AlignSelf::FlexEnd)
        } else if word.try_match("baseline") {
            Ok(AlignSelf::Baseline)
        } else if word.try_match("first") {
            let word: HyphenWord = s.parse()?;
            if word.try_match("baseline") {
                Ok(AlignSelf::FirstBaseline)
            } else {
                Err(word.error())
            }
        } else if word.try_match("last") {
            let word: HyphenWord = s.parse()?;
            if word.try_match("baseline") {
                Ok(AlignSelf::LastBaseline)
            } else {
                Err(word.error())
            }
        } else if word.try_match("stretch") {
            Ok(AlignSelf::Stretch)
        } else if word.try_match("safe") {
            let word: HyphenWord = s.parse()?;
            if word.try_match("center") {
                Ok(AlignSelf::SafeCenter)
            } else {
                Err(word.error())
            }
        } else if word.try_match("unsafe") {
            let word: HyphenWord = s.parse()?;
            if word.try_match("center") {
                Ok(AlignSelf::UnsafeCenter)
            } else {
                Err(word.error())
            }
        } else {
            Err(word.error())
        }
    }
}

impl Parse for BackgroundAttachment {
    fn parse(s: ParseStream) -> syn::Result<Self> {
        let word: HyphenWord = s.parse()?;
        if word.try_match("scroll") {
            Ok(BackgroundAttachment::Scroll)
        } else if word.try_match("fixed") {
            Ok(BackgroundAttachment::Fixed)
        } else if word.try_match("local") {
            Ok(BackgroundAttachment::Local)
        } else {
            Err(word.error())
        }
    }
}

impl Parse for BlendMode {
    fn parse(s: ParseStream) -> syn::Result<Self> {
        let word: HyphenWord = s.parse()?;
        if word.try_match("normal") {
            Ok(BlendMode::Normal)
        } else if word.try_match("multiply") {
            Ok(BlendMode::Multiply)
        } else if word.try_match("screen") {
            Ok(BlendMode::Screen)
        } else if word.try_match("overlay") {
            Ok(BlendMode::Overlay)
        } else if word.try_match("darken") {
            Ok(BlendMode::Darken)
        } else if word.try_match("lighten") {
            Ok(BlendMode::Lighten)
        } else if word.try_match("color-dodge") {
            Ok(BlendMode::ColorDodge)
        } else if word.try_match("color-burn") {
            Ok(BlendMode::ColorBurn)
        } else if word.try_match("hard-light") {
            Ok(BlendMode::HardLight)
        } else if word.try_match("soft-light") {
            Ok(BlendMode::SoftLight)
        } else if word.try_match("difference") {
            Ok(BlendMode::Difference)
        } else if word.try_match("exclusion") {
            Ok(BlendMode::Exclusion)
        } else if word.try_match("hue") {
            Ok(BlendMode::Hue)
        } else if word.try_match("saturation") {
            Ok(BlendMode::Saturation)
        } else if word.try_match("color") {
            Ok(BlendMode::Color)
        } else if word.try_match("luminosity") {
            Ok(BlendMode::Luminosity)
        } else {
            Err(word.error())
        }
    }
}

impl Parse for BackgroundImage {
    fn parse(s: ParseStream) -> syn::Result<Self> {
        let peek = HyphenWord::peek_specific(s);
        if peek.as_ref().map(|s| s.as_str()) == Some("url") {
            let url;
            syn::parenthesized!(url in s);
            let url = url.parse::<syn::LitStr>()?;
            Ok(BackgroundImage::Url(url.value()))
        } else {
            let word: HyphenWord = s.parse()?;
            word.add_expected("url");
            if word.try_match("none") {
                Ok(BackgroundImage::None)
            } else {
                Err(word.error())
            }
        }
    }
}

impl Parse for BackgroundBox {
    fn parse(s: ParseStream) -> syn::Result<Self> {
        let word: HyphenWord = s.parse()?;
        if word.try_match("border-box") {
            Ok(BackgroundBox::BorderBox)
        } else if word.try_match("padding-box") {
            Ok(BackgroundBox::PaddingBox)
        } else if word.try_match("content-box") {
            Ok(BackgroundBox::ContentBox)
        } else {
            Err(word.error())
        }
    }
}

impl Parse for BackgroundPosition {
    fn parse(s: ParseStream) -> syn::Result<Self> {
        let word: HyphenWord = s.parse()?;
        if word.try_match("top") {
            Ok(BackgroundPosition::Top)
        } else if word.try_match("bottom") {
            Ok(BackgroundPosition::Bottom)
        } else if word.try_match("left") {
            Ok(BackgroundPosition::Left)
        } else if word.try_match("right") {
            Ok(BackgroundPosition::Right)
        } else if word.try_match("center") {
            Ok(BackgroundPosition::Center)
        } else {
            Err(word.error())
        }
    }
}

impl Parse for BackgroundRepeat {
    fn parse(s: ParseStream) -> syn::Result<Self> {
        let word: HyphenWord = s.parse()?;
        if word.try_match("repeat-x") {
            Ok(BackgroundRepeat::RepeatX)
        } else if word.try_match("repeat-y") {
            Ok(BackgroundRepeat::RepeatY)
        } else if let Ok(v) = s.parse() {
            Ok(BackgroundRepeat::SingleOrDouble(v))
        } else {
            word.add_expected("repeat");
            word.add_expected("space");
            word.add_expected("round");
            word.add_expected("no-repeat");
            Err(word.error())
        }
    }
}

impl Parse for BgRepeatPart {
    fn parse(s: ParseStream) -> syn::Result<Self> {
        let word: HyphenWord = s.parse()?;
        if word.try_match("repeat") {
            Ok(BgRepeatPart::Repeat)
        } else if word.try_match("space") {
            Ok(BgRepeatPart::Space)
        } else if word.try_match("round") {
            Ok(BgRepeatPart::Round)
        } else if word.try_match("no-repeat") {
            Ok(BgRepeatPart::NoRepeat)
        } else {
            Err(word.error())
        }
    }
}

impl Parse for BackgroundSize {
    fn parse(s: ParseStream) -> syn::Result<Self> {
        let word: HyphenWord = s.parse()?;
        if word.try_match("cover") {
            Ok(BackgroundSize::Cover)
        } else if word.try_match("contain") {
            Ok(BackgroundSize::Contain)
        } else if let Ok(v) = s.parse() {
            Ok(BackgroundSize::SingleOrDouble(v))
        } else {
            word.add_expected("<length>");
            word.add_expected("<percentage>");
            word.add_expected("auto");
            Err(word.error())
        }
    }
}

impl Parse for Border {
    fn parse(s: ParseStream) -> syn::Result<Self> {
        fn line_width_error(span: Span) -> syn::Error {
            syn::Error::new(span, "the border width was specified more than once")
        }
        fn line_style_error(span: Span) -> syn::Error {
            syn::Error::new(span, "the border style was specified more than once")
        }
        fn color_error(span: Span) -> syn::Error {
            syn::Error::new(span, "the border color was specified more than once")
        }
        let mut border = Border::new();
        while !(border.is_full() || finished_rule(s)) {
            let mut matched_something = false; // prevents an infinite loop when no matches
            let width_fork = s.fork();
            match width_fork.parse::<LineWidth>() {
                Ok(line_width) => {
                    if border.has_line_width() {
                        return Err(line_width_error(width_fork.cursor().span()));
                    }
                    matched_something = true;
                    border.line_width = Some(line_width);
                    s.advance_to(&width_fork);
                }
                Err(_) => (),
            }
            let style_fork = s.fork();
            match style_fork.parse::<LineStyle>() {
                Ok(line_style) => {
                    if border.has_line_style() {
                        return Err(line_style_error(style_fork.cursor().span()));
                    }
                    matched_something = true;
                    border.line_style = Some(line_style);
                    s.advance_to(&style_fork);
                }
                Err(_) => (),
            }
            let color_fork = s.fork();
            match color_fork.parse::<Color>() {
                Ok(color) => {
                    if border.has_color() {
                        return Err(color_error(color_fork.cursor().span()));
                    }
                    matched_something = true;
                    border.color = Some(color);
                    s.advance_to(&color_fork);
                }
                Err(_) => (),
            }
            if !(matched_something || finished_rule(s)) {
                return Err(syn::Error::new(
                    s.cursor().span(),
                    "unexpected input - expected one of border-width, border-style, color",
                ));
            }
        }
        Ok(border)
    }
}

#[test]
fn test_border_color() {
    for (input, output) in vec![
        ("black", Rect::All(Color::Black)),
        (
            "#fff blue",
            Rect::VerticalHorizontal(Color::HexRGB(255, 255, 255), Color::Blue),
        ),
        (
            "blue hsl(20, 5%, 100%) white",
            Rect::TopHorizontalBottom(Color::Blue, Color::HSL(20.0, 5.0, 100.0), Color::White),
        ),
        (
            "hsla(20, 5%, 100%, 0.2) #fff #ccc white",
            Rect::TopRightBottomLeft(
                Color::HSLA(20.0, 5.0, 100.0, 0.2),
                Color::HexRGB(255, 255, 255),
                Color::HexRGB(204, 204, 204),
                Color::White,
            ),
        ),
    ] {
        assert_eq!(syn::parse_str::<Rect<Color>>(input).unwrap(), output);
    }
}

#[test]
fn test_border_width() {
    for (input, output) in vec![
        ("1px", BorderWidth::All(LineWidth::Length(Length::Px(1.0)))),
        (
            "1px 2\"em\"",
            BorderWidth::VerticalHorizontal(
                LineWidth::Length(Length::Px(1.0)),
                LineWidth::Length(Length::Em(2.0)),
            ),
        ),
        (
            "2\"em\" medium thick",
            BorderWidth::TopHorizontalBottom(
                LineWidth::Length(Length::Em(2.0)),
                LineWidth::Medium,
                LineWidth::Thick,
            ),
        ),
        (
            "2\"em\" medium 1px thick",
            BorderWidth::TopRightBottomLeft(
                LineWidth::Length(Length::Em(2.0)),
                LineWidth::Medium,
                LineWidth::Length(Length::Px(1.0)),
                LineWidth::Thick,
            ),
        ),
    ] {
        assert_eq!(syn::parse_str::<BorderWidth>(input).unwrap(), output);
    }

    for input in vec!["thi", "1px 1px 1px 1px 1px"] {
        assert!(syn::parse_str::<BorderWidth>(input).is_err());
    }
}

impl Parse for BorderCollapse {
    fn parse(s: ParseStream) -> syn::Result<Self> {
        let word: HyphenWord = s.parse()?;
        if word.try_match("collapse") {
            Ok(BorderCollapse::Collapse)
        } else if word.try_match("separate") {
            Ok(BorderCollapse::Separate)
        } else {
            Err(word.error())
        }
    }
}

impl Parse for BoxShadow {
    fn parse(s: ParseStream) -> syn::Result<Self> {
        syn::custom_keyword!(none);
        if s.peek(none) {
            s.parse::<none>()?;
            Ok(BoxShadow::None)
        } else {
            Ok(BoxShadow::Shadows(s.parse()?))
        }
    }
}

impl Parse for BoxSizing {
    fn parse(s: ParseStream) -> syn::Result<Self> {
        let word: HyphenWord = s.parse()?;
        if word.try_match("border-box") {
            Ok(BoxSizing::BorderBox)
        } else if word.try_match("content-box") {
            Ok(BoxSizing::ContentBox)
        } else {
            Err(word.error())
        }
    }
}

impl Parse for Clear {
    fn parse(s: ParseStream) -> syn::Result<Self> {
        let word: HyphenWord = s.parse()?;
        if word.try_match("none") {
            Ok(Clear::None)
        } else if word.try_match("left") {
            Ok(Clear::Left)
        } else if word.try_match("right") {
            Ok(Clear::Right)
        } else if word.try_match("both") {
            Ok(Clear::Both)
        } else if word.try_match("inline-start") {
            Ok(Clear::InlineStart)
        } else if word.try_match("inline-end") {
            Ok(Clear::InlineEnd)
        } else {
            Err(word.error())
        }
    }
}

impl Parse for ColumnCount {
    fn parse(s: ParseStream) -> syn::Result<Self> {
        if s.peek(syn::LitInt) {
            Ok(ColumnCount::Fixed(s.parse::<Integer<u32>>()?.into_inner()))
        } else {
            let word: HyphenWord = s.parse()?;
            word.add_expected("integer");
            if word.try_match("auto") {
                Ok(ColumnCount::Auto)
            } else {
                Err(word.error())
            }
        }
    }
}

#[test]
fn test_clear() {
    for (input, output) in vec![
        ("none", Clear::None),
        ("left", Clear::Left),
        ("right", Clear::Right),
        ("both", Clear::Both),
        ("inline-start", Clear::InlineStart),
        ("inline-end", Clear::InlineEnd),
    ] {
        assert_eq!(syn::parse_str::<Clear>(input).unwrap(), output);
    }
}

impl Parse for Cursor {
    fn parse(s: ParseStream) -> syn::Result<Self> {
        let word: HyphenWord = s.parse()?;
        if word.try_match("auto") {
            Ok(Cursor::Auto)
        } else if word.try_match("default") {
            Ok(Cursor::Default)
        } else if word.try_match("none") {
            Ok(Cursor::None)
        } else if word.try_match("context-menu") {
            Ok(Cursor::ContextMenu)
        } else if word.try_match("help") {
            Ok(Cursor::Help)
        } else if word.try_match("pointer") {
            Ok(Cursor::Pointer)
        } else if word.try_match("progress") {
            Ok(Cursor::Progress)
        } else if word.try_match("wait") {
            Ok(Cursor::Wait)
        } else if word.try_match("cell") {
            Ok(Cursor::Cell)
        } else if word.try_match("crosshair") {
            Ok(Cursor::Crosshair)
        } else if word.try_match("text") {
            Ok(Cursor::Text)
        } else if word.try_match("vertical-text") {
            Ok(Cursor::VerticalText)
        } else if word.try_match("alias") {
            Ok(Cursor::Alias)
        } else if word.try_match("copy") {
            Ok(Cursor::Copy)
        } else if word.try_match("move") {
            Ok(Cursor::Move)
        } else if word.try_match("no-drop") {
            Ok(Cursor::NoDrop)
        } else if word.try_match("not-allowed") {
            Ok(Cursor::NotAllowed)
        } else if word.try_match("grab") {
            Ok(Cursor::Grab)
        } else if word.try_match("grabbing") {
            Ok(Cursor::Grabbing)
        } else if word.try_match("e-resize") {
            Ok(Cursor::EResize)
        } else if word.try_match("n-resize") {
            Ok(Cursor::NResize)
        } else if word.try_match("ne-resize") {
            Ok(Cursor::NEResize)
        } else if word.try_match("nw-resize") {
            Ok(Cursor::NWResize)
        } else if word.try_match("s-resize") {
            Ok(Cursor::SResize)
        } else if word.try_match("se-resize") {
            Ok(Cursor::SEResize)
        } else if word.try_match("sw-resize") {
            Ok(Cursor::SWResize)
        } else if word.try_match("w-resize") {
            Ok(Cursor::WResize)
        } else if word.try_match("ew-resize") {
            Ok(Cursor::EWResize)
        } else if word.try_match("ns-resize") {
            Ok(Cursor::NSResize)
        } else if word.try_match("nesw-resize") {
            Ok(Cursor::NESWResize)
        } else if word.try_match("nwse-resize") {
            Ok(Cursor::NWSEResize)
        } else if word.try_match("col-resize") {
            Ok(Cursor::ColResize)
        } else if word.try_match("row-resize") {
            Ok(Cursor::RowResize)
        } else if word.try_match("all-scroll") {
            Ok(Cursor::AllScroll)
        } else if word.try_match("zoom-in") {
            Ok(Cursor::ZoomIn)
        } else if word.try_match("zoom-out") {
            Ok(Cursor::ZoomOut)
        } else {
            Err(word.error())
        }
    }
}

impl Parse for Display {
    fn parse(s: ParseStream) -> syn::Result<Self> {
        let word: HyphenWord = s.parse()?;
        if word.try_match("block") {
            Ok(Display::Block)
        } else if word.try_match("flex") {
            Ok(Display::Flex)
        } else if word.try_match("inline") {
            Ok(Display::Inline)
        } else {
            Err(word.error())
        }
    }
}

impl Parse for FlexBasis {
    fn parse(s: ParseStream) -> syn::Result<Self> {
        syn::custom_keyword!(content);

        if s.peek(content) {
            s.parse::<content>()?;
            Ok(FlexBasis::Content)
        } else {
            let w: Width21 = s.parse()?;
            Ok(FlexBasis::Width(w))
        }
    }
}

impl Parse for FlexDirection {
    fn parse(s: ParseStream) -> syn::Result<Self> {
        let word: HyphenWord = s.parse()?;
        if word.try_match("column") {
            Ok(FlexDirection::Column)
        } else if word.try_match("row") {
            Ok(FlexDirection::Row)
        } else {
            Err(word.error())
        }
    }
}

impl Parse for FlexWrap {
    fn parse(s: ParseStream) -> syn::Result<Self> {
        let word: HyphenWord = s.parse()?;
        if word.try_match("wrap") {
            Ok(FlexWrap::Wrap)
        } else if word.try_match("nowrap") {
            Ok(FlexWrap::Nowrap)
        } else {
            Err(word.error())
        }
    }
}

impl Parse for Float {
    fn parse(s: ParseStream) -> syn::Result<Self> {
        let word: HyphenWord = s.parse()?;
        if word.try_match("none") {
            Ok(Float::None)
        } else if word.try_match("left") {
            Ok(Float::Left)
        } else if word.try_match("right") {
            Ok(Float::Right)
        } else if word.try_match("inline-start") {
            Ok(Float::InlineStart)
        } else if word.try_match("inline-end") {
            Ok(Float::InlineEnd)
        } else {
            Err(word.error())
        }
    }
}

impl Parse for Font {
    fn parse(s: ParseStream) -> syn::Result<Self> {
        if s.peek(syn::LitStr) {
            Ok(Font::Named(s.parse::<syn::LitStr>()?.value()))
        } else {
            let name: HyphenWord = s.parse()?;
            name.add_expected("named font");

            if name.try_match("serif") {
                Ok(Font::Serif)
            } else if name.try_match("sans-serif") {
                Ok(Font::SansSerif)
            } else if name.try_match("cursive") {
                Ok(Font::Cursive)
            } else if name.try_match("fantasy") {
                Ok(Font::Fantasy)
            } else if name.try_match("monospace") {
                Ok(Font::Fantasy)
            } else {
                Err(name.error())
            }
        }
    }
}

#[test]
fn test_font_family() {
    for (input, output) in vec![
        (
            "cursive",
            FontFamily {
                first: Font::Cursive,
                rest: vec![],
            },
        ),
        (
            "\"Amatic SC\", sans-serif",
            FontFamily {
                first: Font::Named("Amatic SC".to_string()),
                rest: vec![Font::SansSerif],
            },
        ),
    ] {
        assert_eq!(syn::parse_str::<FontFamily>(input).unwrap(), output);
    }

    for val in vec![
        "font-family:\"Font Awesome 5 Free\"",
        "font-family:\"Some Name\",\"Another Name\",serif",
    ] {
        assert_eq!(&syn::parse_str::<Style>(val).unwrap().to_string(), val);
    }
}

impl Parse for FontSize {
    fn parse(s: ParseStream) -> syn::Result<Self> {
        let word_fork = s.fork();
        let name: HyphenWord = word_fork.parse()?;

        if name.try_match("xx-small") {
            s.advance_to(&word_fork);
            Ok(FontSize::XXSmall)
        } else if name.try_match("x-small") {
            s.advance_to(&word_fork);
            Ok(FontSize::XSmall)
        } else if name.try_match("small") {
            s.advance_to(&word_fork);
            Ok(FontSize::Small)
        } else if name.try_match("medium") {
            s.advance_to(&word_fork);
            Ok(FontSize::Medium)
        } else if name.try_match("large") {
            s.advance_to(&word_fork);
            Ok(FontSize::Large)
        } else if name.try_match("x-large") {
            s.advance_to(&word_fork);
            Ok(FontSize::XLarge)
        } else if name.try_match("xx-large") {
            s.advance_to(&word_fork);
            Ok(FontSize::XXLarge)
        } else if name.try_match("xxx-large") {
            s.advance_to(&word_fork);
            Ok(FontSize::XXXLarge)
        } else if name.try_match("larger") {
            s.advance_to(&word_fork);
            Ok(FontSize::Larger)
        } else if name.try_match("smaller") {
            s.advance_to(&word_fork);
            Ok(FontSize::Smaller)
        } else {
            s.parse().map(FontSize::LengthPercentage).map_err(|_| {
                name.add_expected("length");
                name.add_expected("percentage");
                name.error()
            })
        }
    }
}
impl Parse for FontStyle {
    fn parse(s: ParseStream) -> syn::Result<Self> {
        let name: HyphenWord = s.parse()?;

        if name.try_match("normal") {
            Ok(FontStyle::Normal)
        } else if name.try_match("italic") {
            Ok(FontStyle::Italic)
        } else if name.try_match("oblique") {
            Ok(FontStyle::Oblique)
        } else {
            Err(name.error())
        }
    }
}

#[test]
fn test_font_style() {
    for (input, output) in vec![
        ("normal", FontStyle::Normal),
        ("italic", FontStyle::Italic),
        ("oblique", FontStyle::Oblique),
    ] {
        assert_eq!(syn::parse_str::<FontStyle>(input).unwrap(), output);
    }

    for input in vec!["norma", "normal trailing"] {
        assert!(syn::parse_str::<FontStyle>(input).is_err());
    }
}

impl Parse for FontWeight {
    fn parse(s: ParseStream) -> syn::Result<Self> {
        let name: HyphenWord = s.parse()?;
        name.add_expected("number where 1 <= number <= 1000");

        if name.try_match("normal") {
            Ok(FontWeight::Normal)
        } else if name.try_match("bold") {
            Ok(FontWeight::Bold)
        } else if name.try_match("lighter") {
            Ok(FontWeight::Lighter)
        } else if name.try_match("bolder") {
            Ok(FontWeight::Bolder)
        } else {
            let n: Number = s.parse().map_err(|_| name.error())?;
            if n.suffix.is_empty() && n.value >= 1.0 && n.value <= 1000.0 {
                Ok(FontWeight::Number(n.value))
            } else {
                Err(name.error())
            }
        }
    }
}

#[test]
fn test_font_weight() {
    for (input, output) in vec![
        ("normal", FontWeight::Normal),
        ("bold", FontWeight::Bold),
        ("lighter", FontWeight::Lighter),
        ("bolder", FontWeight::Bolder),
        ("1", FontWeight::Number(1.0)),
        ("1.0", FontWeight::Number(1.0)),
        ("1000", FontWeight::Number(1000.0)),
        ("1000.0", FontWeight::Number(1000.0)),
        ("246.15", FontWeight::Number(246.15)),
    ] {
        match syn::parse_str::<FontWeight>(input) {
            Ok(v) => assert_eq!(v, output),
            Err(e) => panic!("error parsing {}: {}", input, e),
        }
    }
}

impl Parse for JustifyContent {
    fn parse(s: ParseStream) -> syn::Result<Self> {
        let name: HyphenWord = s.parse()?;

        if name.try_match("flex-start") {
            Ok(JustifyContent::FlexStart)
        } else if name.try_match("flex-end") {
            Ok(JustifyContent::FlexEnd)
        } else if name.try_match("center") {
            Ok(JustifyContent::Center)
        } else if name.try_match("space-between") {
            Ok(JustifyContent::SpaceBetween)
        } else if name.try_match("space-around") {
            Ok(JustifyContent::SpaceAround)
        } else if name.try_match("start") {
            // - not in level 1 spec
            Ok(JustifyContent::FlexStart)
        } else if name.try_match("end") {
            // - not in level 1 spec
            Ok(JustifyContent::FlexEnd)
        } else {
            Err(name.error())
        }
    }
}

impl Parse for Length {
    fn parse(s: ParseStream) -> syn::Result<Self> {
        let neg = if s.peek(Token![-]) {
            s.parse::<Token![-]>()?;
            true
        } else {
            false
        };
        let n: Number = s.parse()?;
        Length::parse_from_number(n, neg)
    }
}

impl Length {
    fn parse_from_number(n: Number, neg: bool) -> syn::Result<Self> {
        let neg = if neg { -1.0 } else { 1.0 };
        if n.suffix == "em" {
            Ok(Length::Em(n.value * neg))
        } else if n.suffix == "ex" {
            Ok(Length::Ex(n.value * neg))
        } else if n.suffix == "in" {
            Ok(Length::In(n.value * neg))
        } else if n.suffix == "cm" {
            Ok(Length::Cm(n.value * neg))
        } else if n.suffix == "mm" {
            Ok(Length::Mm(n.value * neg))
        } else if n.suffix == "pt" {
            Ok(Length::Pt(n.value * neg))
        } else if n.suffix == "pc" {
            Ok(Length::Pc(n.value * neg))
        } else if n.suffix == "px" {
            Ok(Length::Px(n.value * neg))
        } else if n.suffix == "" && n.value == 0.0 {
            Ok(Length::Zero)
        } else {
            // No matches so return error
            Err(syn::Error::new(
                n.span,
                "expected one of `\"em\"`, `\"ex\"`, `in`, `cm`, `mm`, `pt`, `pc`, `px` after number, or 0",
            ))
        }
    }
}

impl Parse for LineStyle {
    fn parse(s: ParseStream) -> syn::Result<Self> {
        let name = s.parse::<HyphenWord>()?;
        if name.try_match("none") {
            Ok(LineStyle::None)
        } else if name.try_match("hidden") {
            Ok(LineStyle::Hidden)
        } else if name.try_match("dotted") {
            Ok(LineStyle::Dotted)
        } else if name.try_match("dashed") {
            Ok(LineStyle::Dashed)
        } else if name.try_match("solid") {
            Ok(LineStyle::Solid)
        } else if name.try_match("double") {
            Ok(LineStyle::Double)
        } else if name.try_match("groove") {
            Ok(LineStyle::Groove)
        } else if name.try_match("ridge") {
            Ok(LineStyle::Ridge)
        } else if name.try_match("inset") {
            Ok(LineStyle::Inset)
        } else if name.try_match("outset") {
            Ok(LineStyle::Outset)
        } else {
            Err(name.error())
        }
    }
}

impl Parse for LineWidth {
    fn parse(s: ParseStream) -> syn::Result<Self> {
        let name = s.parse::<HyphenWord>()?;
        if name.try_match("thin") {
            Ok(LineWidth::Thin)
        } else if name.try_match("medium") {
            Ok(LineWidth::Medium)
        } else if name.try_match("thick") {
            Ok(LineWidth::Thick)
        } else {
            match s.parse::<Length>() {
                Ok(l) => Ok(LineWidth::Length(l)),
                Err(_) => {
                    name.add_expected("length");
                    Err(name.error())
                }
            }
        }
    }
}

#[test]
fn test_parse_line_width() {
    assert_eq!(
        syn::parse_str::<LineWidth>("thin").unwrap(),
        LineWidth::Thin
    );
}

impl Parse for LineHeight {
    fn parse(s: ParseStream) -> syn::Result<Self> {
        Ok(LineHeight(s.parse::<syn::LitFloat>()?.base10_parse()?))
    }
}

impl Parse for ListStyleType {
    fn parse(s: ParseStream) -> syn::Result<Self> {
        let name: HyphenWord = s.parse()?;

        if name.try_match("disc") {
            Ok(ListStyleType::Disc)
        } else if name.try_match("circle") {
            Ok(ListStyleType::Circle)
        } else if name.try_match("square") {
            Ok(ListStyleType::Square)
        } else if name.try_match("decimal") {
            Ok(ListStyleType::Decimal)
        } else if name.try_match("decimal-leading-zero") {
            Ok(ListStyleType::DecimalLeadingZero)
        } else if name.try_match("lower-roman") {
            Ok(ListStyleType::LowerRoman)
        } else if name.try_match("upper-roman") {
            Ok(ListStyleType::UpperRoman)
        } else if name.try_match("lower-greek") {
            Ok(ListStyleType::LowerGreek)
        } else if name.try_match("upper-greek") {
            Ok(ListStyleType::UpperGreek)
        } else if name.try_match("lower-latin") {
            Ok(ListStyleType::LowerLatin)
        } else if name.try_match("upper-latin") {
            Ok(ListStyleType::UpperLatin)
        } else if name.try_match("armenian") {
            Ok(ListStyleType::Armenian)
        } else if name.try_match("georgian") {
            Ok(ListStyleType::Georgian)
        } else if name.try_match("lower-alpha") {
            Ok(ListStyleType::LowerAlpha)
        } else if name.try_match("upper-alpha") {
            Ok(ListStyleType::UpperAlpha)
        } else if name.try_match("none") {
            Ok(ListStyleType::None)
        } else {
            Err(name.error())
        }
    }
}

impl Parse for MaxWidthHeight {
    fn parse(s: ParseStream) -> syn::Result<Self> {
        let name = s.parse::<HyphenWord>()?;
        name.add_expected("length");
        name.add_expected("percentage");
        if name.try_match("none") {
            Ok(MaxWidthHeight::None)
        } else if name.try_match("min-content") {
            Ok(MaxWidthHeight::MinContent)
        } else if name.try_match("max-content") {
            Ok(MaxWidthHeight::MaxContent)
        } else if name.try_match("fit-content") {
            let content;
            syn::parenthesized!(content in s);
            Ok(MaxWidthHeight::FitContent(content.parse()?))
        } else {
            s.parse()
                .map(|lp| MaxWidthHeight::LengthPercentage(lp))
                .map_err(|_| name.error())
        }
    }
}

#[test]
fn test_max_width_height() {
    let style: Style = syn::parse_str("max-width: 200px").unwrap();
    assert_eq!(&style.to_string(), "max-width:200px");
}

impl<T> Parse for Rect<T>
where
    T: Parse,
{
    fn parse(s: ParseStream) -> syn::Result<Self> {
        let first = s.parse::<T>()?;
        let fork = s.fork();
        let second = match fork.parse::<T>() {
            Ok(v) => {
                s.advance_to(&fork);
                v
            }
            Err(_) => return Ok(Rect::All(first)),
        };
        let third = match fork.parse::<T>() {
            Ok(v) => {
                s.advance_to(&fork);
                v
            }
            Err(_) => return Ok(Rect::VerticalHorizontal(first, second)),
        };
        match fork.parse::<T>() {
            Ok(v) => {
                s.advance_to(&fork);
                Ok(Rect::TopRightBottomLeft(first, second, third, v))
            }
            Err(_) => Ok(Rect::TopHorizontalBottom(first, second, third)),
        }
    }
}

impl Parse for AutoLengthPercentage {
    fn parse(s: ParseStream) -> syn::Result<Self> {
        syn::custom_keyword!(auto);
        if s.peek(auto) {
            s.parse::<auto>()?;
            Ok(AutoLengthPercentage::Auto)
        } else {
            Ok(AutoLengthPercentage::LengthPercentage(s.parse()?))
        }
    }
}

impl Parse for ObjectFit {
    fn parse(s: ParseStream) -> syn::Result<Self> {
        let name: HyphenWord = s.parse()?;
        if name.try_match("fill") {
            Ok(ObjectFit::Fill)
        } else if name.try_match("none") {
            Ok(ObjectFit::None)
        } else if name.try_match("contain") {
            if s.is_empty() {
                Ok(ObjectFit::Contain { scale_down: false })
            } else {
                let scale_down_word: HyphenWord = s.parse()?;
                if scale_down_word.try_match("scale-down") {
                    Ok(ObjectFit::Contain { scale_down: true })
                } else {
                    Err(scale_down_word.error())
                }
            }
        } else if name.try_match("cover") {
            if HyphenWord::peek(s) {
                let scale_down_word: HyphenWord = s.parse()?;
                if scale_down_word.try_match("scale-down") {
                    Ok(ObjectFit::Cover { scale_down: true })
                } else {
                    Err(scale_down_word.error())
                }
            } else {
                Ok(ObjectFit::Cover { scale_down: false })
            }
        } else if name.try_match("scale-down") {
            if HyphenWord::peek(s) {
                let cover_contain: HyphenWord = s.parse()?;
                if cover_contain.try_match("cover") {
                    Ok(ObjectFit::Cover { scale_down: true })
                } else if cover_contain.try_match("contain") {
                    Ok(ObjectFit::Contain { scale_down: true })
                } else {
                    Err(cover_contain.error())
                }
            } else {
                // defaults to contain when cover/contain not present
                Ok(ObjectFit::Contain { scale_down: true })
            }
        } else {
            Err(name.error())
        }
    }
}

impl Parse for Overflow {
    fn parse(s: ParseStream) -> syn::Result<Self> {
        let first = s.parse::<OverflowXY>()?;
        Ok(match s.parse::<OverflowXY>() {
            Ok(second) => Overflow::XY(first, second),
            Err(_) => Overflow::Both(first),
        })
    }
}

impl Parse for OverflowXY {
    fn parse(s: ParseStream) -> syn::Result<Self> {
        let name: HyphenWord = s.parse()?;

        if name.try_match("visible") {
            Ok(OverflowXY::Visible)
        } else if name.try_match("hidden") {
            Ok(OverflowXY::Hidden)
        } else if name.try_match("clip") {
            Ok(OverflowXY::Clip)
        } else if name.try_match("scroll") {
            Ok(OverflowXY::Scroll)
        } else if name.try_match("auto") {
            Ok(OverflowXY::Auto)
        } else {
            Err(name.error())
        }
    }
}

impl Parse for Position {
    fn parse(s: ParseStream) -> syn::Result<Self> {
        let name: HyphenWord = s.parse()?;
        if name.try_match("static") {
            Ok(Position::Static)
        } else if name.try_match("relative") {
            Ok(Position::Relative)
        } else if name.try_match("absolute") {
            Ok(Position::Absolute)
        } else if name.try_match("fixed") {
            Ok(Position::Fixed)
        } else {
            Err(name.error())
        }
    }
}

#[test]
fn test_padding() {
    for (input, output) in vec![(
        "padding:1\"em\"",
        Style::Padding(Padding::All(Calc::Normal(LengthPercentage::Length(
            Length::Em(1.0),
        )))),
    )] {
        assert_eq!(syn::parse_str::<Style>(input).unwrap(), output);
    }
}

impl Parse for Percentage {
    fn parse(s: ParseStream) -> syn::Result<Self> {
        let n: Number = s.parse()?;
        if n.suffix == "%" {
            Ok(Percentage(n.value))
        } else {
            Err(syn::Error::new(n.span, "expected percentage"))
        }
    }
}

impl Parse for WhiteSpace {
    fn parse(s: ParseStream) -> syn::Result<Self> {
        let name: HyphenWord = s.parse()?;
        if name.try_match("normal") {
            Ok(WhiteSpace::Normal)
        } else if name.try_match("pre") {
            Ok(WhiteSpace::Pre)
        } else if name.try_match("nowrap") {
            Ok(WhiteSpace::Nowrap)
        } else if name.try_match("pre-wrap") {
            Ok(WhiteSpace::PreWrap)
        } else if name.try_match("pre-line") {
            Ok(WhiteSpace::PreLine)
        } else {
            Err(name.error())
        }
    }
}

impl Parse for Width21 {
    fn parse(s: ParseStream) -> syn::Result<Self> {
        syn::custom_keyword!(auto);

        if s.peek(auto) {
            s.parse::<auto>()?;
            Ok(Width21::Auto)
        } else {
            Ok(Width21::LengthPercentage(s.parse()?))
        }
    }
}

impl Parse for WidthHeight {
    fn parse(s: ParseStream) -> syn::Result<Self> {
        let fork = s.fork();
        let name: HyphenWord = fork.parse()?;

        if name.try_match("auto") {
            s.advance_to(&fork);
            Ok(WidthHeight::Auto)
        } else if name.try_match("min-content") {
            s.advance_to(&fork);
            Ok(WidthHeight::MinContent)
        } else if name.try_match("max-content") {
            s.advance_to(&fork);
            Ok(WidthHeight::MaxContent)
        } else if name.try_match("fit-content") {
            s.advance_to(&fork);
            let content;
            syn::parenthesized!(content in s);
            let lp = content.parse()?;
            if !content.is_empty() {
                Err(content.error("trailing tokens"))
            } else {
                Ok(WidthHeight::FitContent(lp))
            }
        } else {
            // todo error message
            Ok(WidthHeight::LengthPercentage(s.parse()?))
        }
    }
}

#[test]
fn test_width_height() {
    for (input, output) in vec![
        ("0", "0"),
        ("1px", "1px"),
        ("1\"em\"", "1em"),
        ("calc(100% - 60px)", "calc(100% - 60px)"),
    ] {
        match syn::parse_str::<WidthHeight>(input) {
            Ok(v) => assert_eq!(&v.to_string(), output),
            Err(e) => panic!("Error in \"{}\": {}", input, e),
        }
    }
}

impl Parse for LengthPercentage {
    fn parse(s: ParseStream) -> syn::Result<Self> {
        if s.peek2(Token![%]) {
            Ok(LengthPercentage::Percentage(s.parse()?))
        } else {
            Ok(LengthPercentage::Length(s.parse()?))
        }
    }
}

#[test]
fn test_length_percentage() {
    for (input, output) in vec![
        ("1\"em\"", LengthPercentage::Length(Length::Em(1.0))),
        ("1.0px", LengthPercentage::Length(Length::Px(1.0))),
        ("0", LengthPercentage::Length(Length::Zero)),
    ] {
        assert_eq!(syn::parse_str::<LengthPercentage>(input).unwrap(), output);
    }
}

impl Parse for Resize {
    fn parse(s: ParseStream) -> syn::Result<Self> {
        let name: HyphenWord = s.parse()?;

        if name.try_match("none") {
            Ok(Resize::None)
        } else if name.try_match("both") {
            Ok(Resize::Both)
        } else if name.try_match("horizontal") {
            Ok(Resize::Horizontal)
        } else if name.try_match("vertical") {
            Ok(Resize::Vertical)
        } else {
            Err(name.error())
        }
    }
}

impl Parse for Shadow {
    fn parse(s: ParseStream) -> syn::Result<Self> {
        syn::custom_keyword!(inset);
        let mut inset_val = false;
        let mut length: Option<ShadowLength> = None;
        let mut color: Option<Color> = None;
        // keep trying all three until we're done or there is an error
        loop {
            let mut parsed_something = false;
            // inset (easiest)
            if s.peek(inset) {
                let inset_tok = s.parse::<inset>()?;
                if inset_val {
                    return Err(syn::Error::new(
                        inset_tok.span(),
                        "`inset` must be specified 0 or 1 times",
                    ));
                }
                inset_val = true;
                parsed_something = true;
            }

            // color
            let fork = s.fork();
            if let Ok(parsed_color) = fork.parse::<Color>() {
                if color.is_some() {
                    return Err(s.error("color must be specified 0 or 1 times"));
                }
                color = Some(parsed_color);
                s.advance_to(&fork);
                parsed_something = true;
            }

            // length
            let fork = s.fork();
            if let Ok(parsed_length) = fork.parse::<ShadowLength>() {
                if length.is_some() {
                    return Err(s.error("shadow length must be specified once"));
                }
                length = Some(parsed_length);
                s.advance_to(&fork);
                parsed_something = true;
            }

            // if we've failed to parse anything, end the loop.
            if !parsed_something {
                break;
            }
        }
        if let Some(length) = length {
            Ok(Shadow {
                color,
                length,
                inset: inset_val,
            })
        } else {
            Err(s.error("expected color, length, or `inset`"))
        }
    }
}

impl Parse for ShadowLength {
    fn parse(s: ParseStream) -> syn::Result<Self> {
        let horizontal: Length = s.parse()?;
        let vertical: Length = s.parse()?;

        // blur
        let fork = s.fork();
        let blur = match fork.parse::<Length>() {
            Ok(blur) => {
                s.advance_to(&fork);
                blur
            }
            Err(_) => {
                return Ok(ShadowLength::Offsets {
                    horizontal,
                    vertical,
                });
            }
        };

        // spread
        let fork = s.fork();
        match fork.parse::<Length>() {
            Ok(spread) => {
                s.advance_to(&fork);

                Ok(ShadowLength::OffsetsBlurSpread {
                    horizontal,
                    vertical,
                    blur,
                    spread,
                })
            }
            Err(_) => Ok(ShadowLength::OffsetsBlur {
                horizontal,
                vertical,
                blur,
            }),
        }
    }
}

impl Parse for TextAlign {
    fn parse(s: ParseStream) -> syn::Result<Self> {
        let word: HyphenWord = s.parse()?;
        if word.try_match("left") {
            Ok(TextAlign::Left)
        } else if word.try_match("right") {
            Ok(TextAlign::Right)
        } else if word.try_match("center") {
            Ok(TextAlign::Center)
        } else if word.try_match("justify") {
            Ok(TextAlign::Justify)
        } else {
            Err(word.error())
        }
    }
}

// color
// =====

impl Parse for DynamicColor {
    fn parse(s: ParseStream) -> syn::Result<Self> {
        Ok(if s.peek(syn::token::Brace) {
            DynamicColor::Dynamic(s.parse()?)
        } else {
            DynamicColor::Literal(s.parse()?)
        })
    }
}

impl Parse for Color {
    fn parse(s: ParseStream) -> syn::Result<Self> {
        if s.peek(Token![#]) {
            return parse_hex_color(s);
        }
        let fn_name: HyphenWord = s.parse()?;
        if fn_name.try_match("hsl") {
            parse_hsl_color(s, false)
        } else if fn_name.try_match("hsla") {
            parse_hsl_color(s, true)
        } else {
            if let Some(name) = fn_name.word.as_ref() {
                if let Some(color) = Color::from_named(name) {
                    return Ok(color);
                }
            }
            fn_name.add_expected("named color");
            Err(fn_name.error())
        }
    }
}

fn parse_hex_color(s: ParseStream) -> syn::Result<Color> {
    const ERR_MSG: &'static str = "to avoid confusing rust, please enclose hex colors in `\"`";
    s.parse::<Token![#]>()?;
    if !(s.peek(syn::LitStr) || s.peek(Ident)) {
        return Err(s.error(ERR_MSG));
    }
    if s.peek(syn::LitStr) {
        let hex_str: syn::LitStr = s.parse()?;
        color::parse_hex(&hex_str.value()).ok_or(syn::Error::new(hex_str.span(), ERR_MSG))
    } else {
        let hex_str: Ident = s.parse()?;
        color::parse_hex(&hex_str.to_string()).ok_or(syn::Error::new(hex_str.span(), ERR_MSG))
    }
}

fn parse_hsl_color(s: ParseStream, with_alpha: bool) -> syn::Result<Color> {
    let content;
    syn::parenthesized!(content in s);
    let n: Number = content.parse()?;
    n.empty_suffix()?;
    let hue = n.value;
    if hue < 0.0 || hue >= 360.0 {
        return Err(syn::Error::new(
            n.span,
            "hue should be in the range `0 <= hue < 360`",
        ));
    }
    content.parse::<Token![,]>()?;
    let n: Number = content.parse()?;
    if n.suffix != "%" {
        return Err(syn::Error::new(
            n.span,
            "saturation should be a percentage (followed by `%`)",
        ));
    }
    let sat = n.value;
    if sat < 0.0 || sat > 100.0 {
        return Err(syn::Error::new(
            n.span,
            "saturation should be in the range `0 <= sat < 100`",
        ));
    }
    content.parse::<Token![,]>()?;
    let n: Number = content.parse()?;
    if n.suffix != "%" {
        return Err(syn::Error::new(
            n.span,
            "saturation should be a percentage (followed by `%`)",
        ));
    }
    let light = n.value;
    if light < 0.0 || light > 100.0 {
        return Err(syn::Error::new(
            n.span,
            "lightness should be in the range `0 <= light < 100`",
        ));
    }
    // since we parse content in parentheses, we can assume no trailing characers
    if !with_alpha {
        return if content.is_empty() {
            Ok(Color::HSL(hue, sat, light))
        } else {
            Err(content.error("trailing characters"))
        };
    }
    // we are a hsla
    content.parse::<Token![,]>()?;
    let n: Number = content.parse()?;
    n.empty_suffix()?;
    let alpha = n.value;
    if alpha < 0.0 || alpha > 1.0 {
        return Err(syn::Error::new(
            n.span,
            "alpha should be in the range `0 <= alpha < 1`",
        ));
    }
    if content.is_empty() {
        Ok(Color::HSLA(hue, sat, light, alpha))
    } else {
        Err(content.error("unexpected trailing characters"))
    }
}

#[test]
fn test_color() {
    for (input, output) in vec![
        ("#ffffffff", Color::HexRGBA(255, 255, 255, 255)),
        ("#ffffff", Color::HexRGB(255, 255, 255)),
        ("#fff", Color::HexRGB(255, 255, 255)),
        ("#\"fff\"", Color::HexRGB(255, 255, 255)),
        ("hsl(100, 50%, 50%)", Color::HSL(100.0, 50.0, 50.0)),
        ("hsla(60, 0%, 0%, 0.2)", Color::HSLA(60.0, 0.0, 0.0, 0.2)),
        ("black", Color::Black),
        ("yellow", Color::Yellow),
    ] {
        match syn::parse_str::<Color>(input) {
            Ok(c) => assert_eq!(c, output),
            Err(e) => panic!("error parsing color {}: {}", input, e),
        }
    }
}

// Util
// ====

impl<T> Parse for NonemptyCommaList<T>
where
    T: Parse,
{
    fn parse(s: ParseStream) -> syn::Result<Self> {
        let punctuated = Punctuated::<T, Token![,]>::parse_separated_nonempty(s)?;
        let mut iter = punctuated.into_iter();
        let first = iter.next().unwrap();
        Ok(Self {
            first,
            rest: iter.collect(),
        })
    }
}

impl<T> Parse for SingleOrDouble<T>
where
    T: Parse,
{
    fn parse(s: ParseStream) -> syn::Result<Self> {
        let first = T::parse(s)?;
        let fork = s.fork();
        Ok(match T::parse(&fork) {
            Ok(second) => {
                s.advance_to(&fork);
                SingleOrDouble::Double {
                    vert: first,
                    horiz: second,
                }
            }
            Err(_) => SingleOrDouble::Single(first),
        })
    }
}

/// Either a float or an int, converted in either case to f64.
///
/// A trailing percent (`%`) character will be consumed if the number has no suffix. This is valid
/// according to the CSS tokeniser spec.
///
/// TODO This only works for floats for now. Although JS only supports floats, integer literals are
/// used in css.
#[derive(Debug)]
struct Number {
    value: f64,
    suffix: String,
    span: Span,
}

impl Number {
    fn empty_suffix(&self) -> syn::Result<()> {
        if self.suffix != "" {
            Err(syn::Error::new(
                self.span,
                "unexpected characters after number",
            ))
        } else {
            Ok(())
        }
    }

    #[cfg(test)]
    fn check_value(&self, value: f64, suffix: &str) -> bool {
        self.value == value && self.suffix == suffix
    }
}

impl Parse for Number {
    fn parse(s: ParseStream) -> syn::Result<Number> {
        let lookahead = s.lookahead1();
        let (value, mut span, mut suffix) = if lookahead.peek(syn::LitFloat) {
            let tok = s.parse::<syn::LitFloat>()?;
            let num = tok.base10_parse()?;
            (num, tok.span(), tok.suffix().to_string())
        } else if lookahead.peek(syn::LitInt) {
            let tok = s.parse::<syn::LitInt>()?;
            // u32 chosen because it can be safely converted into f64
            let num = tok.base10_parse::<u32>()?;
            (num.into(), tok.span(), tok.suffix().to_string())
        } else {
            return Err(lookahead.error());
        };
        if suffix.is_empty() {
            // look for a `%` for the suffix
            if s.peek(Token![%]) {
                let tok = s.parse::<Token![%]>()?;
                if let Some(extra_span) = span.join(tok.span) {
                    span = extra_span;
                }
                suffix.push('%');
            // work-around using literal strings because the lexer can't support suffixes beginning
            // with `e` for floats: https://github.com/rust-lang/rust/issues/67544
            } else if s.peek(syn::LitStr) {
                let tok = s.parse::<syn::LitStr>()?;
                if let Some(extra_span) = span.join(tok.span()) {
                    span = extra_span;
                }
                suffix.push_str(&tok.value());
            }
        }
        Ok(Number {
            value,
            suffix,
            span,
        })
    }
}

#[test]
fn test_number() {
    for (input, value, suffix) in vec![
        ("200", 200.0, ""),
        ("200.0", 200.0, ""),
        ("0", 0.0, ""),
        ("0in", 0.0, "in"),
    ] {
        assert!(syn::parse_str::<Number>(input)
            .unwrap()
            .check_value(value, suffix),)
    }
}

/// Something like `word-separated-hyphens`
#[derive(Debug)]
struct HyphenWord {
    pub span: Span,
    pub word: Option<String>,
    /// List of tried matches - for building error.
    tried: TryList,
}

impl HyphenWord {
    pub fn new(span: Span, word: String) -> Self {
        HyphenWord {
            span,
            word: Some(word),
            tried: TryList::new(),
        }
    }

    /// This allows HyphenWords to be empty. In this case the token cursor will not advance and the
    /// returned word will be blank.
    pub fn new_no_word(span: Span) -> Self {
        HyphenWord {
            span,
            word: None,
            tried: TryList::new(),
        }
    }

    pub fn try_match(&self, other: &str) -> bool {
        if Some(other) == self.word.as_ref().map(|s| s.as_str()) {
            true
        } else {
            self.tried.add_literal(other);
            false
        }
    }

    pub fn add_expected(&self, ty: &str) {
        self.tried.add(ty);
    }

    /// Panics if there were no calls to `try_match` before calling this function.
    pub fn error(&self) -> syn::Error {
        self.tried.to_error(self.span)
    }

    /// This is cheaper than peek-specific
    pub fn peek(s: ParseStream) -> bool {
        s.peek(Ident)
    }

    /// Peek the next HyphenWord without advancing the parser.
    pub fn peek_specific(s: ParseStream) -> Option<String> {
        let fork = s.fork();
        match HyphenWord::parse(&fork) {
            Ok(hw) => Some(hw.word.unwrap()),
            Err(_) => None,
        }
    }
}

impl Parse for HyphenWord {
    fn parse(s: ParseStream) -> syn::Result<Self> {
        let fork = s.fork();
        let first = match fork.call(Ident::parse_any) {
            Ok(v) => {
                s.advance_to(&fork);
                v
            }
            Err(_) => return Ok(HyphenWord::new_no_word(s.cursor().span())),
        };
        let mut word = first.to_string();
        let mut span = first.span();
        // This is potentially unbounded. Probably not be a problem but making a note anyway.
        while s.peek(Token![-]) {
            let hyphen = s.parse::<Token![-]>()?;
            if let Some(joined) = span.join(hyphen.span) {
                span = joined;
            }
            let part = s.call(Ident::parse_any)?;
            write!(word, "-{}", part).unwrap();
            if let Some(joined) = span.join(part.span()) {
                span = joined;
            }
        }
        Ok(HyphenWord::new(span, word))
    }
}

#[test]
fn test_hyphen_word() {
    let word: HyphenWord = syn::parse_str("first-second-third").unwrap();
    assert_eq!(word.word, Some("first-second-third".to_string()));
    assert!(syn::parse_str::<HyphenWord>("first-second-").is_err());
    assert!(syn::parse_str::<HyphenWord>("a a").is_err());
}

/// Keeps track of a list of tokens that have been tried.
#[derive(Debug)]
pub struct TryList(RefCell<BTreeSet<String>>);

impl TryList {
    pub fn new() -> Self {
        TryList(RefCell::new(BTreeSet::new()))
    }

    /// Same as add, but with quotes
    pub fn add_literal(&self, lit: &str) {
        self.add(format!("`{}`", lit));
    }

    pub fn add(&self, ty: impl Into<String>) {
        self.0.borrow_mut().insert(ty.into());
    }

    fn to_error(&self, span: Span) -> syn::Error {
        let tried = self.0.borrow();
        let mut iter = tried.iter();
        let start = iter.next().unwrap().to_owned();
        let list = iter.fold(start, |mut acc, itm| {
            write!(acc, ", {}", itm).unwrap();
            acc
        });
        let error_msg = format!("expected one of {}", list);
        syn::Error::new(span, error_msg)
    }
}

/// Whether we are at the end of a rule. Either the stream will be empty, or there will be a
/// semi-colon.
fn finished_rule(s: ParseStream) -> bool {
    s.is_empty() || s.peek(Token![;])
}

// Parsing integers

#[derive(Debug, PartialEq)]
struct Integer<T> {
    value: T,
}

impl<T> Integer<T> {
    fn into_inner(self) -> T {
        self.value
    }
}

impl<T> Parse for Integer<T>
where
    T: str::FromStr + fmt::Display + PartialOrd<T>,
    <T as str::FromStr>::Err: fmt::Display,
{
    fn parse(s: ParseStream) -> syn::Result<Self> {
        Ok(Integer {
            value: integer(s, ..)?,
        })
    }
}

/// Parse an integer, with an optional allowed range.
fn integer<T, R>(s: ParseStream, range: R) -> syn::Result<T>
where
    R: RangeBounds<T> + fmt::Debug,
    T: str::FromStr + fmt::Display + PartialOrd<T>,
    <T as str::FromStr>::Err: fmt::Display,
{
    let fixed = s.parse::<syn::LitInt>()?;
    let span = fixed.span();
    if fixed.suffix().is_empty() {
        let fixed = fixed.base10_parse()?;
        if range.contains(&fixed) {
            Ok(fixed)
        } else {
            Err(syn::Error::new(
                span,
                format!(
                    "expected a number in the range {:?}, found {}",
                    range, fixed
                ),
            ))
        }
    } else {
        Err(syn::Error::new(span, "the number should not have a suffix"))
    }
}

#[test]
fn test_parse_integer() {
    let x: Integer<u8> = syn::parse_str("123").unwrap();
    assert_eq!(x.into_inner(), 123);
    let x: syn::Result<Integer<u8>> = syn::parse_str("256");
    assert!(x.is_err());
}

// tests

#[test]
fn downstream_bug1() {
    let s: Styles = syn::parse_str(
        "display: flex;
        flex-direction: column;
        flex-grow: 1;
        flex-shrink: 0;",
    )
    .unwrap();
    assert_eq!(
        s.rules,
        vec![
            Style::Display(Display::Flex),
            Style::FlexDirection(FlexDirection::Column),
            Style::FlexGrow(1.0),
            Style::FlexShrink(0.0)
        ]
    )
}

#[test]
#[ignore]
fn inline_logic() {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Use if you want to test that something parses, but not if it looks the same when
    /// stringified. Example "border: 1px" -> "border:1px" but we still might want to check that
    /// the former parses.
    fn parse(input: &str) -> Style {
        syn::parse_str(input).unwrap()
    }

    /// This function can be used to quickly write tests to check that a parse and a stringify are
    /// opposites.
    fn round_trip_style(input: &str) {
        assert_eq!(&parse(input).to_string(), input);
    }

    #[test]
    fn border_bottom_left_radius() {
        round_trip_style("border-bottom-left-radius:30% 3px");
    }

    #[test]
    fn border_bottom_right_radius() {
        round_trip_style("border-bottom-right-radius:0 0");
    }

    #[test]
    fn border_collapse() {
        round_trip_style("border-collapse:collapse");
    }

    #[test]
    fn border_width() {
        round_trip_style("border-width:1px");
        round_trip_style("border-width:0 2px 50pt 0");
    }
}
