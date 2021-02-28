//! A module to type styles.
// TODO most stuff here is on the stack, but there are a few heap-allocs here and there. It would
// be good if we could just to allocate them in the bump arena when using bumpalo.
mod calc;
mod codegen;
mod color;
pub mod string;
mod syn_parse;

use std::{
    fmt,
    ops::{Deref, DerefMut},
};

pub use calc::*;
pub use color::{Color, DynamicColor};
// pub use crate::{
//     calc::*,
//     color::{Color, DynamicColor},
// };

pub struct DynamicStyles {
    pub rules: Vec<DynamicStyle>,
}

impl From<Vec<DynamicStyle>> for DynamicStyles {
    fn from(v: Vec<DynamicStyle>) -> Self {
        Self { rules: v }
    }
}

impl fmt::Display for DynamicStyles {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for style in self
            .rules
            .iter()
            .filter(|style| !(style.is_dynamic() || style.is_dummy()))
        {
            write!(f, "{};", style)?;
        }
        Ok(())
    }
}

// TODO make container generic over heap (e.g. support bumpalo)
#[derive(Debug, Clone, PartialEq)]
pub struct Styles {
    pub rules: Vec<Style>,
}

impl Styles {
    pub fn new() -> Self {
        Styles { rules: Vec::new() }
    }

    pub fn add(&mut self, style: Style) {
        self.rules.push(style);
    }

    pub fn merge(&mut self, other: Styles) {
        self.rules.extend(other.rules.into_iter())
    }
}

impl From<DynamicStyles> for Styles {
    fn from(dy: DynamicStyles) -> Self {
        Styles {
            rules: dy
                .rules
                .into_iter()
                .filter_map(|dy_sty| match dy_sty {
                    DynamicStyle::Dynamic(_) => None,
                    DynamicStyle::Literal(l) => Some(l),
                })
                .collect(),
        }
    }
}

impl Deref for Styles {
    type Target = Vec<Style>;
    fn deref(&self) -> &Self::Target {
        &self.rules
    }
}
impl DerefMut for Styles {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.rules
    }
}

impl From<Vec<Style>> for Styles {
    fn from(v: Vec<Style>) -> Self {
        Self { rules: v }
    }
}

impl From<Styles> for Vec<Style> {
    fn from(v: Styles) -> Self {
        v.rules
    }
}

impl fmt::Display for Styles {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for style in self.rules.iter().filter(|style| !style.is_dummy()) {
            write!(f, "{};", style)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum DynamicStyle {
    /// A literal style.
    Literal(Style),
    /// Tokens to pass through directly to codegen.
    Dynamic(syn::Block),
}

impl DynamicStyle {
    pub fn is_dynamic(&self) -> bool {
        match self {
            DynamicStyle::Literal(style) => style.is_dynamic(),
            DynamicStyle::Dynamic(_) => true,
        }
    }
    pub fn is_dummy(&self) -> bool {
        match self {
            DynamicStyle::Literal(style) => style.is_dummy(),
            DynamicStyle::Dynamic(_) => false,
        }
    }
}

impl From<Style> for DynamicStyle {
    fn from(style: Style) -> Self {
        DynamicStyle::Literal(style)
    }
}

impl fmt::Display for DynamicStyle {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            DynamicStyle::Literal(style) => style.fmt(f),
            DynamicStyle::Dynamic(_) => Ok(()),
        }
    }
}

/// a `Style` is one of the css key/value pairs, also sometimes known as rules.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Style {
    /// For when you don't want to include any style at all (useful in expressions like `if`)
    Dummy,
    /// For when you want to use some unimplemented css. This is not type checked!
    Unchecked(String),

    // *From w3 spec:*
    /// align-content
    AlignContent(AlignContent),
    /// align-items
    AlignItems(AlignItems),
    /// align-self
    AlignSelf(AlignSelf),
    // all - todo when doing global values
    // background
    /// background-attachment
    BackgroundAttachment(BackgroundAttachment),
    /// background-blend-mode
    BackgroundBlendMode(NonemptyCommaList<BlendMode>),
    /// background-clip
    BackgroundClip(BackgroundBox),
    /// background-color
    BackgroundColor(DynamicColor),
    /// background-image
    BackgroundImage(NonemptyCommaList<BackgroundImage>),
    /// background-origin
    BackgroundOrigin(BackgroundBox),
    /// background-position
    BackgroundPosition(BackgroundPosition),
    /// background-repeat
    BackgroundRepeat(NonemptyCommaList<BackgroundRepeat>),
    /// background-size
    BackgroundSize(BackgroundSize),
    /// border
    Border(Border),
    /// border-bottom
    BorderBottom(Border),
    /// border-bottom-color
    BorderBottomColor(Color),
    /// border-bottom-left-radius
    BorderBottomLeftRadius(SingleOrDouble<LengthPercentage>),
    /// border-bottom-right-radius
    BorderBottomRightRadius(SingleOrDouble<LengthPercentage>),
    /// border-bottom-style
    BorderBottomStyle(LineStyle),
    /// border-bottom-width
    BorderBottomWidth(LineWidth),
    /// border-collapse
    BorderCollapse(BorderCollapse),
    /// border-color
    BorderColor(Rect<Color>),
    // border-image
    // border-image-outset
    // border-image-repeat
    // border-image-slice
    // border-image-source
    // border-image-width
    /// border-left
    BorderLeft(Border),
    /// border-left-color
    BorderLeftColor(Color),
    /// border-left-style
    BorderLeftStyle(LineStyle),
    /// border-left-width
    BorderLeftWidth(LineWidth),
    /// border-radius
    BorderRadius(BorderRadius),
    /// border-right
    BorderRight(Border),
    /// border-right-color
    BorderRightColor(Color),
    /// border-right-style
    BorderRightStyle(LineStyle),
    /// border-right-width
    BorderRightWidth(LineWidth),
    // border-spacing
    /// border-style
    BorderStyle(BorderStyle),
    /// border-top
    BorderTop(Border),
    /// border-top-color
    BorderTopColor(Color),
    /// border-top-left-radius
    BorderTopLeftRadius(SingleOrDouble<LengthPercentage>),
    /// border-top-right-radius
    BorderTopRightRadius(SingleOrDouble<LengthPercentage>),
    /// border-top-style
    BorderTopStyle(LineStyle),
    /// border-top-width
    BorderTopWidth(LineWidth),
    /// border-width
    BorderWidth(BorderWidth),
    /// bottom
    Bottom(AutoLengthPercentage),
    // box-decoration-break
    /// box-shadow
    BoxShadow(BoxShadow),
    /// box-sizing
    BoxSizing(BoxSizing),
    // break-after
    // break-before
    // break-inside
    // caption-side
    // caret-color
    /// clear
    Clear(Clear),
    // clip
    // clip-path
    // clip-rule
    /// color
    Color(DynamicColor),
    /// column-count (manually added)
    ColumnCount(ColumnCount),
    // contain
    // content
    // counter-increment
    // counter-reset
    // cue
    // cue-after
    // cue-before
    /// cursor
    Cursor(Cursor),
    // direction
    /// display https://www.w3.org/TR/css-display-3/#typedef-display-outside
    Display(Display),
    // elevation
    // empty-cells
    // flex
    /// flex-basis
    FlexBasis(FlexBasis),
    /// flex-direction
    FlexDirection(FlexDirection),
    // flex-flow
    /// flex-grow
    FlexGrow(f64),
    /// flex-shrink
    FlexShrink(f64),
    /// flex-wrap
    FlexWrap(FlexWrap),
    /// float
    Float(Float),
    // font
    /// font-family
    FontFamily(FontFamily),
    // font-feature-settings
    // font-kerning
    /// font-size
    FontSize(FontSize),
    // font-size-adjust
    // font-stretch
    /// font-style
    FontStyle(FontStyle),
    // font-synthesis
    // font-variant
    // font-variant-caps
    // font-variant-east-asian
    // font-variant-ligatures
    // font-variant-numeric
    // font-variant-position
    /// font-weight
    FontWeight(FontWeight),
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
    /// height
    Height(WidthHeight),
    // image-orientation
    // image-rendering
    // isolation
    /// justify-content
    JustifyContent(JustifyContent),
    /// left
    Left(AutoLengthPercentage),
    // letter-spacing
    /// line-height
    LineHeight(LineHeight),
    // list-style
    // list-style-image
    // list-style-position
    /// list-style-type
    ListStyleType(ListStyleType),
    /// margin
    Margin(Margin),
    /// margin-bottom
    MarginBottom(MarginWidth),
    /// margin-left
    MarginLeft(MarginWidth),
    /// margin-right
    MarginRight(MarginWidth),
    /// margin-top
    MarginTop(MarginWidth),
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
    /// max-height
    MaxHeight(MaxWidthHeight),
    /// max-width
    MaxWidth(MaxWidthHeight),
    /// min-height - current implementing CSS2 spec
    MinHeight(Calc),
    /// min-width - current implementing CSS2 spec
    MinWidth(Calc),
    // mix-blend-mode
    /// object-fit - https://drafts.csswg.org/css-images-4/#the-object-fit
    ObjectFit(ObjectFit),
    // object-position
    // opacity
    // order
    // orphans
    // outline
    // outline-color
    // outline-offset
    // outline-style
    // outline-width
    /// overflow - https://drafts.csswg.org/css-overflow-3/#propdef-overflow
    Overflow(Overflow),
    /// overflow-x manually added
    OverflowX(OverflowXY),
    /// overflow-y manually added
    OverflowY(OverflowXY),
    /// padding
    Padding(Padding),
    /// padding-bottom
    PaddingBottom(PaddingWidth),
    /// padding-left
    PaddingLeft(PaddingWidth),
    /// padding-right
    PaddingRight(PaddingWidth),
    /// padding-top
    PaddingTop(PaddingWidth),
    // page-break-after
    // page-break-before
    // page-break-inside
    // pause
    // pause-after
    // pause-before
    // pitch
    // pitch-range
    // play-during
    /// position
    Position(Position),
    // quotes
    /// resize
    Resize(Resize),
    // richness
    /// right
    Right(AutoLengthPercentage),
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
    /// text-align
    TextAlign(TextAlign),
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
    /// top
    Top(AutoLengthPercentage),
    // transform
    // transform-box
    // transform-origin
    // unicode-bidi
    // vertical-align
    // visibility
    // voice-family
    // volume
    /// white-space
    WhiteSpace(WhiteSpace),
    /// widows
    Widows(u32),
    /// width
    Width(WidthHeight),
    // will-change
    // word-spacing
    // writing-mode
    // z-index
}

impl Style {
    fn is_dummy(&self) -> bool {
        match self {
            Style::Dummy => true,
            _ => false,
        }
    }

    fn is_dynamic(&self) -> bool {
        match self {
            Style::BackgroundColor(value) => value.is_dynamic(),
            Style::Color(value) => value.is_dynamic(),
            _ => false,
        }
    }
}

impl fmt::Display for Style {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Style::Dummy => Ok(()),
            Style::Unchecked(v) => write!(f, "{}", v),

            Style::AlignContent(v) => write!(f, "align-content:{}", v),
            Style::AlignItems(v) => write!(f, "align-items:{}", v),
            Style::AlignSelf(v) => write!(f, "align-self:{}", v),
            // all - deferred
            // background
            Style::BackgroundAttachment(v) => write!(f, "background-attachment:{}", v),
            Style::BackgroundBlendMode(v) => write!(f, "background-blend-mode:{}", v),
            Style::BackgroundClip(v) => write!(f, "background-clip:{}", v),
            Style::BackgroundColor(v) => write!(f, "background-color:{}", v),
            Style::BackgroundImage(v) => write!(f, "background-image:{}", v),
            Style::BackgroundOrigin(v) => write!(f, "background-origin:{}", v),
            Style::BackgroundPosition(v) => write!(f, "background-position:{}", v),
            Style::BackgroundRepeat(v) => write!(f, "background-repeat:{}", v),
            Style::BackgroundSize(v) => write!(f, "background-size:{}", v),
            Style::Border(v) => write!(f, "border:{}", v),
            Style::BorderBottom(v) => write!(f, "border-bottom:{}", v),
            Style::BorderBottomColor(v) => write!(f, "border-bottom-color:{}", v),
            Style::BorderBottomLeftRadius(v) => write!(f, "border-bottom-left-radius:{}", v),
            Style::BorderBottomRightRadius(v) => write!(f, "border-bottom-right-radius:{}", v),
            Style::BorderBottomStyle(v) => write!(f, "border-bottom-style:{}", v),
            Style::BorderBottomWidth(v) => write!(f, "border-bottom-width:{}", v),
            Style::BorderCollapse(v) => write!(f, "border-collapse:{}", v),
            Style::BorderColor(v) => write!(f, "border-color:{}", v),
            // border-image
            // border-image-outset
            // border-image-repeat
            // border-image-slice
            // border-image-source
            // border-image-width
            Style::BorderLeft(v) => write!(f, "border-left:{}", v),
            Style::BorderLeftColor(v) => write!(f, "border-left-color:{}", v),
            Style::BorderLeftStyle(v) => write!(f, "border-left-style:{}", v),
            Style::BorderLeftWidth(v) => write!(f, "border-left-width:{}", v),
            Style::BorderRadius(v) => write!(f, "border-radius:{}", v),
            Style::BorderRight(v) => write!(f, "border-right:{}", v),
            Style::BorderRightColor(v) => write!(f, "border-right-color:{}", v),
            Style::BorderRightStyle(v) => write!(f, "border-right-style:{}", v),
            Style::BorderRightWidth(v) => write!(f, "border-right-width:{}", v),
            // border-spacing
            Style::BorderStyle(v) => write!(f, "border-style:{}", v),
            Style::BorderTop(v) => write!(f, "border-top:{}", v),
            Style::BorderTopColor(v) => write!(f, "border-top-color:{}", v),
            Style::BorderTopLeftRadius(v) => write!(f, "border-top-left-radius:{}", v),
            Style::BorderTopRightRadius(v) => write!(f, "border-top-right-radius:{}", v),
            Style::BorderTopStyle(v) => write!(f, "border-top-style:{}", v),
            Style::BorderTopWidth(v) => write!(f, "border-top-width:{}", v),
            Style::BorderWidth(v) => write!(f, "border-width:{}", v),
            Style::Bottom(v) => write!(f, "bottom:{}", v),
            // box-decoration-break
            Style::BoxShadow(v) => write!(f, "box-shadow:{}", v),
            Style::BoxSizing(v) => write!(f, "box-sizing:{}", v),
            // break-after
            // break-before
            // break-inside
            // caption-side
            // caret-color
            Style::Clear(v) => write!(f, "clear:{}", v),
            // clip
            // clip-path
            // clip-rule
            Style::Color(v) => write!(f, "color:{}", v),
            Style::ColumnCount(v) => write!(f, "column-count:{}", v),
            // contain
            // content
            // counter-increment
            // counter-reset
            // cue
            // cue-after
            // cue-before
            Style::Cursor(v) => write!(f, "cursor:{}", v),
            // direction
            Style::Display(v) => write!(f, "display:{}", v),
            // elevation
            // empty-cells
            // flex
            Style::FlexBasis(v) => write!(f, "flex-basis:{}", v),
            Style::FlexDirection(v) => write!(f, "flex-direction:{}", v),
            // flex-flow
            Style::FlexGrow(v) => write!(f, "flex-grow:{}", v),
            Style::FlexShrink(v) => write!(f, "flex-shrink:{}", v),
            Style::FlexWrap(v) => write!(f, "flex-wrap:{}", v),
            Style::Float(v) => write!(f, "float:{}", v),
            // font
            Style::FontFamily(v) => write!(f, "font-family:{}", v),
            // font-feature-settings
            // font-kerning
            Style::FontSize(v) => write!(f, "font-size:{}", v),
            // font-size-adjust
            // font-stretch
            Style::FontStyle(v) => write!(f, "font-style:{}", v),
            // font-synthesis
            // font-variant
            // font-variant-caps
            // font-variant-east-asian
            // font-variant-ligatures
            // font-variant-numeric
            // font-variant-position
            Style::FontWeight(v) => write!(f, "font-weight:{}", v),
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
            Style::Height(v) => write!(f, "height:{}", v),
            // image-orientation
            // image-rendering
            // isolation
            Style::JustifyContent(v) => write!(f, "justify-content:{}", v),
            // left
            Style::Left(v) => write!(f, "left:{}", v),
            // letter-spacing
            // line-height
            Style::LineHeight(v) => write!(f, "line-height:{}", v),
            // list-style
            // list-style-image
            // list-style-position
            Style::ListStyleType(v) => write!(f, "list-style-type:{}", v),
            Style::Margin(v) => write!(f, "margin:{}", v),
            Style::MarginBottom(v) => write!(f, "margin-bottom:{}", v),
            Style::MarginLeft(v) => write!(f, "margin-left:{}", v),
            Style::MarginRight(v) => write!(f, "margin-right:{}", v),
            Style::MarginTop(v) => write!(f, "margin-top:{}", v),
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
            Style::MaxHeight(v) => write!(f, "max-height:{}", v),
            Style::MaxWidth(v) => write!(f, "max-width:{}", v),
            Style::MinHeight(v) => write!(f, "min-height:{}", v),
            Style::MinWidth(v) => write!(f, "min-width:{}", v),
            // mix-blend-mode
            Style::ObjectFit(v) => write!(f, "object-fit:{}", v),
            // object-position
            // opacity
            // order
            // orphans
            // outline
            // outline-color
            // outline-offset
            // outline-style
            // outline-width
            Style::Overflow(v) => write!(f, "overflow:{}", v),
            Style::OverflowX(v) => write!(f, "overflow-x:{}", v),
            Style::OverflowY(v) => write!(f, "overflow-y:{}", v),
            Style::Padding(v) => write!(f, "padding:{}", v),
            Style::PaddingBottom(v) => write!(f, "padding-bottom:{}", v),
            Style::PaddingLeft(v) => write!(f, "padding-left:{}", v),
            Style::PaddingRight(v) => write!(f, "padding-right:{}", v),
            Style::PaddingTop(v) => write!(f, "padding-top:{}", v),
            // padding-bottom
            // padding-left
            // padding-right
            // padding-top
            // page-break-after
            // page-break-before
            // page-break-inside
            // pause
            // pause-after
            // pause-before
            // pitch
            // pitch-range
            // play-during
            Style::Position(v) => write!(f, "position:{}", v),
            // uotes
            Style::Resize(v) => write!(f, "resize:{}", v),
            // richness
            Style::Right(v) => write!(f, "right:{}", v),
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
            Style::TextAlign(v) => write!(f, "text-align:{}", v),
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
            // top
            Style::Top(v) => write!(f, "top:{}", v),
            // transform
            // transform-box
            // transform-origin
            // unicode-bidi
            // vertical-align
            // visibility
            // voice-family
            // volume
            Style::WhiteSpace(v) => write!(f, "white-space:{}", v),
            Style::Widows(v) => write!(f, "widows:{}", v),
            Style::Width(v) => write!(f, "width:{}", v),
            // will-change
            // word-spacing
            // writing-mode
            // z-index
        }
    }
}

/// https://www.w3.org/TR/css-flexbox-1/#propdef-justify-content
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AlignContent {
    FlexStart,
    Center,
    FlexEnd,
    SpaceBetween,
    SpaceAround,
    Stretch,
}

impl Default for AlignContent {
    fn default() -> Self {
        AlignContent::Stretch
    }
}

impl fmt::Display for AlignContent {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AlignContent::FlexStart => write!(f, "flex-start"),
            AlignContent::Center => write!(f, "center"),
            AlignContent::FlexEnd => write!(f, "flex-end"),
            AlignContent::SpaceAround => write!(f, "space-around"),
            AlignContent::SpaceBetween => write!(f, "space-between"),
            AlignContent::Stretch => write!(f, "stretch"),
        }
    }
}

/// https://developer.mozilla.org/en-US/docs/Web/CSS/align-items
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AlignItems {
    Normal,
    Stretch,
    Center,
    Start,
    End,
    FlexStart,
    FlexEnd,
    Baseline,
    FirstBaseline,
    LastBaseline,
    SafeCenter,
    UnsafeCenter,
}

impl fmt::Display for AlignItems {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AlignItems::Normal => write!(f, "normal"),
            AlignItems::Stretch => write!(f, "stretch"),
            AlignItems::Center => write!(f, "center"),
            AlignItems::Start => write!(f, "start"),
            AlignItems::End => write!(f, "end"),
            AlignItems::FlexStart => write!(f, "flex-start"),
            AlignItems::FlexEnd => write!(f, "flex-end"),
            AlignItems::Baseline => write!(f, "baseline"),
            AlignItems::FirstBaseline => write!(f, "first baseline"),
            AlignItems::LastBaseline => write!(f, "last baseline"),
            AlignItems::SafeCenter => write!(f, "safe center"),
            AlignItems::UnsafeCenter => write!(f, "unsafe center"),
        }
    }
}

/// https://developer.mozilla.org/en-US/docs/Web/CSS/align-self
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AlignSelf {
    Auto,
    Normal,
    Center,
    Start,
    End,
    SelfStart,
    SelfEnd,
    FlexStart,
    FlexEnd,
    Baseline,
    FirstBaseline,
    LastBaseline,
    Stretch,
    SafeCenter,
    UnsafeCenter,
}

impl fmt::Display for AlignSelf {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AlignSelf::Auto => write!(f, "auto"),
            AlignSelf::Normal => write!(f, "normal"),
            AlignSelf::Center => write!(f, "center"),
            AlignSelf::Start => write!(f, "start"),
            AlignSelf::End => write!(f, "end"),
            AlignSelf::SelfStart => write!(f, "self-start"),
            AlignSelf::SelfEnd => write!(f, "self-end"),
            AlignSelf::FlexStart => write!(f, "flex-start"),
            AlignSelf::FlexEnd => write!(f, "flex-end"),
            AlignSelf::Baseline => write!(f, "baseline"),
            AlignSelf::FirstBaseline => write!(f, "first baseline"),
            AlignSelf::LastBaseline => write!(f, "last baseline"),
            AlignSelf::Stretch => write!(f, "stretch"),
            AlignSelf::SafeCenter => write!(f, "safe center"),
            AlignSelf::UnsafeCenter => write!(f, "unsafe center"),
        }
    }
}

/// https://developer.mozilla.org/en-US/docs/Web/CSS/background-attachment
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BackgroundAttachment {
    Scroll,
    Fixed,
    Local,
}

impl fmt::Display for BackgroundAttachment {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            BackgroundAttachment::Scroll => write!(f, "scroll"),
            BackgroundAttachment::Fixed => write!(f, "fixed"),
            BackgroundAttachment::Local => write!(f, "local"),
        }
    }
}

/// https://developer.mozilla.org/en-US/docs/Web/CSS/background-blend-mode
#[derive(Debug, Clone, PartialEq)]
pub enum BlendMode {
    Normal,
    Multiply,
    Screen,
    Overlay,
    Darken,
    Lighten,
    ColorDodge,
    ColorBurn,
    HardLight,
    SoftLight,
    Difference,
    Exclusion,
    Hue,
    Saturation,
    Color,
    Luminosity,
}

impl fmt::Display for BlendMode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            BlendMode::Normal => write!(f, "normal"),
            BlendMode::Multiply => write!(f, "multiply"),
            BlendMode::Screen => write!(f, "screen"),
            BlendMode::Overlay => write!(f, "overlay"),
            BlendMode::Darken => write!(f, "darken"),
            BlendMode::Lighten => write!(f, "lighten"),
            BlendMode::ColorDodge => write!(f, "color-dodge"),
            BlendMode::ColorBurn => write!(f, "color-burn"),
            BlendMode::HardLight => write!(f, "hard-light"),
            BlendMode::SoftLight => write!(f, "soft-light"),
            BlendMode::Difference => write!(f, "difference"),
            BlendMode::Exclusion => write!(f, "exclusion"),
            BlendMode::Hue => write!(f, "hue"),
            BlendMode::Saturation => write!(f, "saturation"),
            BlendMode::Color => write!(f, "color"),
            BlendMode::Luminosity => write!(f, "luminosity"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum BackgroundBox {
    BorderBox,
    PaddingBox,
    ContentBox,
}

impl fmt::Display for BackgroundBox {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            BackgroundBox::BorderBox => write!(f, "border-box"),
            BackgroundBox::PaddingBox => write!(f, "padding-box"),
            BackgroundBox::ContentBox => write!(f, "content-box"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum BackgroundImage {
    None,
    Url(String),
    // TODO other variants
}

impl fmt::Display for BackgroundImage {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            BackgroundImage::None => write!(f, "none"),
            BackgroundImage::Url(url) => write!(f, "url({})", url),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum BackgroundPosition {
    Top,
    Bottom,
    Left,
    Right,
    Center,
    // TODO other variants
}

impl fmt::Display for BackgroundPosition {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            BackgroundPosition::Top => write!(f, "top"),
            BackgroundPosition::Left => write!(f, "left"),
            BackgroundPosition::Bottom => write!(f, "bottom"),
            BackgroundPosition::Right => write!(f, "right"),
            BackgroundPosition::Center => write!(f, "center"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum BackgroundRepeat {
    RepeatX,
    RepeatY,
    SingleOrDouble(SingleOrDouble<BgRepeatPart>),
}

impl fmt::Display for BackgroundRepeat {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            BackgroundRepeat::RepeatX => write!(f, "repeat-x"),
            BackgroundRepeat::RepeatY => write!(f, "repeat-y"),
            BackgroundRepeat::SingleOrDouble(v) => v.fmt(f),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum BgRepeatPart {
    Repeat,
    Space,
    Round,
    NoRepeat,
}

impl fmt::Display for BgRepeatPart {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            BgRepeatPart::Repeat => write!(f, "repeat"),
            BgRepeatPart::Space => write!(f, "space"),
            BgRepeatPart::Round => write!(f, "round"),
            BgRepeatPart::NoRepeat => write!(f, "no-repeat"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum BackgroundSize {
    Cover,
    Contain,
    SingleOrDouble(SingleOrDouble<AutoLengthPercentage>),
}

impl fmt::Display for BackgroundSize {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            BackgroundSize::Cover => write!(f, "cover"),
            BackgroundSize::Contain => write!(f, "contain"),
            BackgroundSize::SingleOrDouble(v) => v.fmt(f),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Border {
    pub line_width: Option<LineWidth>,
    pub line_style: Option<LineStyle>,
    pub color: Option<Color>,
}

impl Border {
    fn new() -> Self {
        Border {
            line_width: None,
            line_style: None,
            color: None,
        }
    }

    fn is_full(&self) -> bool {
        match (&self.line_width, &self.line_style, &self.color) {
            (Some(_), Some(_), Some(_)) => true,
            _ => false,
        }
    }

    fn has_line_width(&self) -> bool {
        self.line_width.is_some()
    }
    fn has_line_style(&self) -> bool {
        self.line_style.is_some()
    }
    fn has_color(&self) -> bool {
        self.color.is_some()
    }
}

impl fmt::Display for Border {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fn space(yes: bool) -> &'static str {
            if yes {
                " "
            } else {
                ""
            }
        }
        let mut cont = false;
        if let Some(line_width) = self.line_width {
            write!(f, "{}", line_width)?;
            cont = true;
        }
        if let Some(line_style) = self.line_style {
            write!(f, "{}{}", space(cont), line_style)?;
            cont = true;
        }
        if let Some(color) = self.color {
            write!(f, "{}{}", space(cont), color)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum BorderCollapse {
    Collapse,
    Separate,
}

impl fmt::Display for BorderCollapse {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            BorderCollapse::Collapse => write!(f, "collapse"),
            BorderCollapse::Separate => write!(f, "separate"),
        }
    }
}

pub type BorderRadius = Calc;

pub type BorderStyle = Rect<LineStyle>;

pub type BorderWidth = Rect<LineWidth>;

#[derive(Debug, Clone, PartialEq)]
pub enum BoxShadow {
    None,
    Shadows(NonemptyCommaList<Shadow>),
}

impl fmt::Display for BoxShadow {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            BoxShadow::None => f.write_str("none"),
            BoxShadow::Shadows(list) => write!(f, "{}", list),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BoxSizing {
    BorderBox,
    ContentBox,
}

impl fmt::Display for BoxSizing {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            BoxSizing::BorderBox => f.write_str("border-box"),
            BoxSizing::ContentBox => f.write_str("content-box"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Clear {
    None,
    Left,
    Right,
    Both,
    InlineStart,
    InlineEnd,
}

impl fmt::Display for Clear {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Clear::None => f.write_str("none"),
            Clear::Left => f.write_str("left"),
            Clear::Right => f.write_str("right"),
            Clear::Both => f.write_str("both"),
            Clear::InlineStart => f.write_str("inline-start"),
            Clear::InlineEnd => f.write_str("inline-end"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ColumnCount {
    Auto,
    Fixed(u32),
}

impl fmt::Display for ColumnCount {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ColumnCount::Auto => f.write_str("auto"),
            ColumnCount::Fixed(inner) => write!(f, "{}", inner),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Cursor {
    // todo url
    Auto,
    Default,
    None,
    ContextMenu,
    Help,
    Pointer,
    Progress,
    Wait,
    Cell,
    Crosshair,
    Text,
    VerticalText,
    Alias,
    Copy,
    Move,
    NoDrop,
    NotAllowed,
    Grab,
    Grabbing,
    EResize,
    NResize,
    NEResize,
    NWResize,
    SResize,
    SEResize,
    SWResize,
    WResize,
    EWResize,
    NSResize,
    NESWResize,
    NWSEResize,
    ColResize,
    RowResize,
    AllScroll,
    ZoomIn,
    ZoomOut,
}

impl fmt::Display for Cursor {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Cursor::Auto => f.write_str("auto"),
            Cursor::Default => f.write_str("default"),
            Cursor::None => f.write_str("none"),
            Cursor::ContextMenu => f.write_str("context-menu"),
            Cursor::Help => f.write_str("help"),
            Cursor::Pointer => f.write_str("pointer"),
            Cursor::Progress => f.write_str("progress"),
            Cursor::Wait => f.write_str("wait"),
            Cursor::Cell => f.write_str("cell"),
            Cursor::Crosshair => f.write_str("crosshair"),
            Cursor::Text => f.write_str("text"),
            Cursor::VerticalText => f.write_str("vertical-text"),
            Cursor::Alias => f.write_str("alias"),
            Cursor::Copy => f.write_str("copy"),
            Cursor::Move => f.write_str("move"),
            Cursor::NoDrop => f.write_str("no-drop"),
            Cursor::NotAllowed => f.write_str("not-allowed"),
            Cursor::Grab => f.write_str("grab"),
            Cursor::Grabbing => f.write_str("grabbing"),
            Cursor::EResize => f.write_str("e-resize"),
            Cursor::NResize => f.write_str("n-resize"),
            Cursor::NEResize => f.write_str("ne-resize"),
            Cursor::NWResize => f.write_str("nw-resize"),
            Cursor::SResize => f.write_str("s-resize"),
            Cursor::SEResize => f.write_str("se-resize"),
            Cursor::SWResize => f.write_str("sw-resize"),
            Cursor::WResize => f.write_str("w-resize"),
            Cursor::EWResize => f.write_str("ew-resize"),
            Cursor::NSResize => f.write_str("ns-resize"),
            Cursor::NESWResize => f.write_str("nesw-resize"),
            Cursor::NWSEResize => f.write_str("nwse-resize"),
            Cursor::ColResize => f.write_str("col-resize"),
            Cursor::RowResize => f.write_str("row-resize"),
            Cursor::AllScroll => f.write_str("all-scroll"),
            Cursor::ZoomIn => f.write_str("zoom-in"),
            Cursor::ZoomOut => f.write_str("zoom-out"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Display {
    Block,
    Flex,
    Inline,
    // todo incomplete
}

impl fmt::Display for Display {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Display::Block => f.write_str("block"),
            Display::Flex => f.write_str("flex"),
            Display::Inline => f.write_str("inline"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum FlexBasis {
    Width(Width21),
    Content,
}

impl fmt::Display for FlexBasis {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            FlexBasis::Width(v) => fmt::Display::fmt(v, f),
            FlexBasis::Content => f.write_str("content"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FlexDirection {
    Row,
    Column,
}

impl fmt::Display for FlexDirection {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            FlexDirection::Row => f.write_str("row"),
            FlexDirection::Column => f.write_str("column"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FlexWrap {
    Wrap,
    Nowrap,
}

impl fmt::Display for FlexWrap {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            FlexWrap::Wrap => write!(f, "wrap"),
            FlexWrap::Nowrap => write!(f, "nowrap"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Float {
    None,
    Left,
    Right,
    InlineStart,
    InlineEnd,
}

impl fmt::Display for Float {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Float::None => f.write_str("inline-end"),
            Float::Left => f.write_str("left"),
            Float::Right => f.write_str("right"),
            Float::InlineStart => f.write_str("inline-start"),
            Float::InlineEnd => f.write_str("inline-end"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Font {
    // todo escape when `Display`ing
    Named(String),
    Serif,
    SansSerif,
    Cursive,
    Fantasy,
    Monospace,
}

impl fmt::Display for Font {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Font::Named(inner) => write!(f, "\"{}\"", inner),
            Font::Serif => write!(f, "serif"),
            Font::SansSerif => write!(f, "sans-serif"),
            Font::Cursive => write!(f, "cursive"),
            Font::Fantasy => write!(f, "fantasy"),
            Font::Monospace => write!(f, "monospace"),
        }
    }
}

pub type FontFamily = NonemptyCommaList<Font>;

#[derive(Debug, Clone, PartialEq)]
pub enum FontSize {
    XXSmall,
    XSmall,
    Small,
    Medium,
    Large,
    XLarge,
    XXLarge,
    XXXLarge,
    Larger,
    Smaller,
    LengthPercentage(Calc),
}

impl fmt::Display for FontSize {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            FontSize::XXSmall => f.write_str("xx-small"),
            FontSize::XSmall => f.write_str("x-small"),
            FontSize::Small => f.write_str("small"),
            FontSize::Medium => f.write_str("medium"),
            FontSize::Large => f.write_str("large"),
            FontSize::XLarge => f.write_str("x-large"),
            FontSize::XXLarge => f.write_str("xx-large"),
            FontSize::XXXLarge => f.write_str("xxx-large"),
            FontSize::Larger => f.write_str("larger"),
            FontSize::Smaller => f.write_str("smaller"),
            FontSize::LengthPercentage(v) => fmt::Display::fmt(v, f),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FontStyle {
    Normal,
    Italic,
    Oblique,
}

impl fmt::Display for FontStyle {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            FontStyle::Normal => f.write_str("normal"),
            FontStyle::Italic => f.write_str("italic"),
            FontStyle::Oblique => f.write_str("oblique"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FontWeight {
    Normal,
    Bold,
    Lighter,
    Bolder,
    /// Between 1 and 1000
    Number(f64),
}

impl fmt::Display for FontWeight {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            FontWeight::Normal => f.write_str("normal"),
            FontWeight::Bold => f.write_str("bold"),
            FontWeight::Lighter => f.write_str("lighter"),
            FontWeight::Bolder => f.write_str("bolder"),
            FontWeight::Number(v) => fmt::Display::fmt(v, f),
        }
    }
}

/// https://www.w3.org/TR/css-flexbox-1/#propdef-justify-content
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum JustifyContent {
    FlexStart,
    Center,
    FlexEnd,
    SpaceBetween,
    SpaceAround,
}

impl fmt::Display for JustifyContent {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            JustifyContent::FlexStart => write!(f, "flex-start"),
            JustifyContent::Center => write!(f, "center"),
            JustifyContent::FlexEnd => write!(f, "flex-end"),
            JustifyContent::SpaceAround => write!(f, "space-around"),
            JustifyContent::SpaceBetween => write!(f, "space-between"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Length {
    Em(f64),
    Ex(f64),
    In(f64),
    Cm(f64),
    Mm(f64),
    Pt(f64),
    Pc(f64),
    Px(f64),
    Zero,
}

impl fmt::Display for Length {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Length::Em(val) => write!(f, "{}em", val),
            Length::Ex(val) => write!(f, "{}ex", val),
            Length::In(val) => write!(f, "{}in", val),
            Length::Cm(val) => write!(f, "{}cm", val),
            Length::Mm(val) => write!(f, "{}mm", val),
            Length::Pt(val) => write!(f, "{}pt", val),
            Length::Pc(val) => write!(f, "{}pc", val),
            Length::Px(val) => write!(f, "{}px", val),
            Length::Zero => write!(f, "0"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LengthPercentage {
    Length(Length),
    Percentage(Percentage),
}

impl fmt::Display for LengthPercentage {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            LengthPercentage::Length(v) => fmt::Display::fmt(v, f),
            LengthPercentage::Percentage(v) => fmt::Display::fmt(v, f),
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LineStyle {
    None,
    Hidden,
    Dotted,
    Dashed,
    Solid,
    Double,
    Groove,
    Ridge,
    Inset,
    Outset,
}

impl fmt::Display for LineStyle {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            LineStyle::None => write!(f, "none"),
            LineStyle::Hidden => write!(f, "hidden"),
            LineStyle::Dotted => write!(f, "dotted"),
            LineStyle::Dashed => write!(f, "dashed"),
            LineStyle::Solid => write!(f, "solid"),
            LineStyle::Double => write!(f, "double"),
            LineStyle::Groove => write!(f, "groove"),
            LineStyle::Ridge => write!(f, "ridge"),
            LineStyle::Inset => write!(f, "inset"),
            LineStyle::Outset => write!(f, "outset"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LineWidth {
    Length(Length),
    Thin,
    Medium,
    Thick,
}

impl fmt::Display for LineWidth {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            LineWidth::Length(v) => fmt::Display::fmt(v, f),
            LineWidth::Thin => write!(f, "thin"),
            LineWidth::Medium => write!(f, "medium"),
            LineWidth::Thick => write!(f, "thick"),
        }
    }
}

// TODO this isn't the full spec for lineheight
// (https://www.w3.org/TR/CSS2/visudet.html#propdef-line-height)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LineHeight(f64);

impl fmt::Display for LineHeight {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ListStyleType {
    Disc,
    Circle,
    Square,
    Decimal,
    DecimalLeadingZero,
    LowerRoman,
    UpperRoman,
    LowerGreek,
    UpperGreek,
    LowerLatin,
    UpperLatin,
    Armenian,
    Georgian,
    LowerAlpha,
    UpperAlpha,
    None,
}

impl fmt::Display for ListStyleType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ListStyleType::Disc => write!(f, "disc"),
            ListStyleType::Circle => write!(f, "circle"),
            ListStyleType::Square => write!(f, "square"),
            ListStyleType::Decimal => write!(f, "decimal"),
            ListStyleType::DecimalLeadingZero => write!(f, "decimal-leading-zero"),
            ListStyleType::LowerRoman => write!(f, "lower-roman"),
            ListStyleType::UpperRoman => write!(f, "upper-roman"),
            ListStyleType::LowerGreek => write!(f, "lower-greek"),
            ListStyleType::UpperGreek => write!(f, "upper-greek"),
            ListStyleType::LowerLatin => write!(f, "lower-latin"),
            ListStyleType::UpperLatin => write!(f, "upper-latin"),
            ListStyleType::Armenian => write!(f, "armenian"),
            ListStyleType::Georgian => write!(f, "georgian"),
            ListStyleType::LowerAlpha => write!(f, "lower-alpha"),
            ListStyleType::UpperAlpha => write!(f, "upper-alpha"),
            ListStyleType::None => write!(f, "none"),
        }
    }
}

pub type Margin = Rect<MarginWidth>;

#[derive(Debug, Clone, PartialEq)]
pub enum AutoLengthPercentage {
    LengthPercentage(Calc),
    Auto,
}

impl fmt::Display for AutoLengthPercentage {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AutoLengthPercentage::LengthPercentage(v) => fmt::Display::fmt(v, f),
            AutoLengthPercentage::Auto => write!(f, "auto"),
        }
    }
}

pub type MarginWidth = AutoLengthPercentage;

/// for max-width and max-height
#[derive(Debug, Clone, PartialEq)]
pub enum MaxWidthHeight {
    None,
    LengthPercentage(Calc),
    MinContent,
    MaxContent,
    FitContent(Calc),
}

impl fmt::Display for MaxWidthHeight {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            MaxWidthHeight::None => write!(f, "none"),
            MaxWidthHeight::LengthPercentage(v) => write!(f, "{}", v),
            MaxWidthHeight::MinContent => write!(f, "min-content"),
            MaxWidthHeight::MaxContent => write!(f, "max-content"),
            MaxWidthHeight::FitContent(v) => write!(f, "fit-content({})", v),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ObjectFit {
    Fill,
    None,
    Contain { scale_down: bool },
    Cover { scale_down: bool },
}

impl fmt::Display for ObjectFit {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ObjectFit::Fill => write!(f, "fill"),
            ObjectFit::None => write!(f, "none"),
            ObjectFit::Contain { scale_down } => {
                if *scale_down {
                    write!(f, "contain scale-down")
                } else {
                    write!(f, "contain")
                }
            }
            ObjectFit::Cover { scale_down } => {
                if *scale_down {
                    write!(f, "cover scale-down")
                } else {
                    write!(f, "cover")
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Overflow {
    Both(OverflowXY),
    XY(OverflowXY, OverflowXY),
}

impl fmt::Display for Overflow {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Overflow::Both(v) => write!(f, "{}", v),
            Overflow::XY(x, y) => write!(f, "{} {}", x, y),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OverflowXY {
    Visible,
    Hidden,
    Clip,
    Scroll,
    Auto,
}

impl fmt::Display for OverflowXY {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            OverflowXY::Visible => write!(f, "visible"),
            OverflowXY::Hidden => write!(f, "hidden"),
            OverflowXY::Clip => write!(f, "clip"),
            OverflowXY::Scroll => write!(f, "scroll"),
            OverflowXY::Auto => write!(f, "auto"),
        }
    }
}

pub type Padding = Rect<Calc>;

/// for e.g. `padding-top`
pub type PaddingWidth = Calc;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Percentage(pub f64);

impl fmt::Display for Percentage {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}%", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Position {
    Static,
    Relative,
    Absolute,
    Fixed,
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Position::Static => write!(f, "static"),
            Position::Relative => write!(f, "relative"),
            Position::Absolute => write!(f, "absolute"),
            Position::Fixed => write!(f, "fixed"),
        }
    }
}

/// For parsing things in groups of 1, 2, 3 or 4 for specifying the sides of a rectangle.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Rect<T> {
    All(T),
    VerticalHorizontal(T, T),
    TopHorizontalBottom(T, T, T),
    TopRightBottomLeft(T, T, T, T),
}

impl<T> fmt::Display for Rect<T>
where
    T: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Rect::All(a) => write!(f, "{}", a),
            Rect::VerticalHorizontal(v, h) => write!(f, "{} {}", v, h),
            Rect::TopHorizontalBottom(t, h, b) => write!(f, "{} {} {}", t, h, b),
            Rect::TopRightBottomLeft(t, r, b, l) => write!(f, "{} {} {} {}", t, r, b, l),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Resize {
    None,
    Both,
    Horizontal,
    Vertical,
}

impl fmt::Display for Resize {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Resize::None => write!(f, "none"),
            Resize::Both => write!(f, "both"),
            Resize::Horizontal => write!(f, "horizontal"),
            Resize::Vertical => write!(f, "vertical"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Shadow {
    pub color: Option<Color>,
    pub length: ShadowLength,
    pub inset: bool,
}

impl fmt::Display for Shadow {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // we do it in this order because it makes spacing easier.
        write!(f, "{}", self.length)?;
        if let Some(color) = self.color {
            write!(f, " {}", color)?;
        }
        if self.inset {
            write!(f, " inset")?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ShadowLength {
    Offsets {
        horizontal: Length,
        vertical: Length,
    },
    OffsetsBlur {
        horizontal: Length,
        vertical: Length,
        blur: Length,
    },
    OffsetsBlurSpread {
        horizontal: Length,
        vertical: Length,
        blur: Length,
        spread: Length,
    },
}

impl fmt::Display for ShadowLength {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ShadowLength::Offsets {
                horizontal,
                vertical,
            } => write!(f, "{} {}", horizontal, vertical),
            ShadowLength::OffsetsBlur {
                horizontal,
                vertical,
                blur,
            } => write!(f, "{} {} {}", horizontal, vertical, blur),
            ShadowLength::OffsetsBlurSpread {
                horizontal,
                vertical,
                blur,
                spread,
            } => write!(f, "{} {} {} {}", horizontal, vertical, blur, spread),
        }
    }
}

#[test]
fn test_shadow_length() {
    for (input, output) in vec![
        (
            "0 10px",
            ShadowLength::Offsets {
                horizontal: Length::Zero,
                vertical: Length::Px(10.0),
            },
        ),
        (
            "0 10px -10px",
            ShadowLength::OffsetsBlur {
                horizontal: Length::Zero,
                vertical: Length::Px(10.0),
                blur: Length::Px(-10.0),
            },
        ),
    ] {
        assert_eq!(syn::parse_str::<ShadowLength>(input).unwrap(), output)
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum TextAlign {
    Left,
    Right,
    Center,
    Justify,
}

impl fmt::Display for TextAlign {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TextAlign::Left => write!(f, "left"),
            TextAlign::Right => write!(f, "right"),
            TextAlign::Center => write!(f, "center"),
            TextAlign::Justify => write!(f, "justify"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Url {
    // todo modifiers
    url: String,
}

impl fmt::Display for Url {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "url(\"{}\")", self.url)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum WhiteSpace {
    Normal,
    Pre,
    Nowrap,
    PreWrap,
    PreLine,
}

impl fmt::Display for WhiteSpace {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            WhiteSpace::Normal => write!(f, "normal"),
            WhiteSpace::Pre => write!(f, "pre"),
            WhiteSpace::Nowrap => write!(f, "nowrap"),
            WhiteSpace::PreWrap => write!(f, "pre-wrap"),
            WhiteSpace::PreLine => write!(f, "pre-line"),
        }
    }
}

/// values of `width` and `height`, `min-width`, `min-height`.
#[derive(Debug, Clone, PartialEq)]
pub enum WidthHeight {
    Auto,
    LengthPercentage(Calc),
    MinContent,
    MaxContent,
    FitContent(Calc),
}

impl fmt::Display for WidthHeight {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            WidthHeight::Auto => write!(f, "auto"),
            WidthHeight::LengthPercentage(v) => write!(f, "{}", v),
            WidthHeight::MinContent => write!(f, "min-content"),
            WidthHeight::MaxContent => write!(f, "max-content"),
            WidthHeight::FitContent(v) => write!(f, "fit-content({})", v),
        }
    }
}

/// CSS2.1 width, for use with flexbox.
#[derive(Debug, Clone, PartialEq)]
pub enum Width21 {
    Auto,
    LengthPercentage(Calc),
}

impl fmt::Display for Width21 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Width21::Auto => write!(f, "auto"),
            Width21::LengthPercentage(v) => fmt::Display::fmt(v, f),
        }
    }
}

/// A generic container for a non-empty comma-separated list of values
#[derive(Debug, Clone, PartialEq)]
pub struct NonemptyCommaList<T> {
    first: T,
    rest: Vec<T>,
}

impl<T> fmt::Display for NonemptyCommaList<T>
where
    T: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.first)?;
        for t in &self.rest {
            write!(f, ",{}", t)?;
        }
        Ok(())
    }
}

/// Matches one or two variables.
#[derive(Debug, Clone, PartialEq)]
pub enum SingleOrDouble<T> {
    Single(T),
    Double { horiz: T, vert: T },
}

impl<T> fmt::Display for SingleOrDouble<T>
where
    T: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SingleOrDouble::Single(t) => t.fmt(f),
            SingleOrDouble::Double { vert, horiz } => write!(f, "{} {}", vert, horiz),
        }
    }
}
