use crate::*;
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};

macro_rules! path {
    ($($t:tt)+) => {
        ::quote::quote!(::style:: $($t)+)
    };
}

impl ToTokens for DynamicStyles {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let parts = self
            .rules
            .iter()
            .filter(|style| !style.is_dummy())
            .map(|style| style.to_token_stream());
        tokens.extend(quote! {
            {
                let mut styles = style::Styles::new();
                #(styles.push(#parts);)*
                styles
            }
        })
    }
}

impl ToTokens for DynamicStyle {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(match self {
            DynamicStyle::Dynamic(block) => quote!(#block),
            DynamicStyle::Literal(lit) => quote!(#lit),
        })
    }
}

impl ToTokens for Style {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let path = quote!(::style::Style::);
        tokens.extend(match self {
            Style::Dummy => quote!(#path Dummy),
            Style::Unchecked(v) => quote!(#path Unchecked(String::from(#v))),

            Style::AlignContent(v) => quote!(#path AlignContent(#v)),
            Style::AlignItems(v) => quote!(#path AlignItems(#v)),
            Style::AlignSelf(v) => quote!(#path AlignSelf(#v)),
            // all
            // background
            Style::BackgroundAttachment(v) => quote!(#path BackgroundAttachment(#v)),
            Style::BackgroundBlendMode(v) => quote!(#path BackgroundBlendMode(#v)),
            Style::BackgroundClip(v) => quote!(#path BackgroundClip(#v)),
            Style::BackgroundColor(v) => quote!(#path BackgroundColor(#v)),
            Style::BackgroundImage(v) => quote!(#path BackgroundImage(#v)),
            Style::BackgroundOrigin(v) => quote!(#path BackgroundOrigin(#v)),
            Style::BackgroundPosition(v) => quote!(#path BackgroundPosition(#v)),
            Style::BackgroundRepeat(v) => quote!(#path BackgroundRepeat(#v)),
            Style::BackgroundSize(v) => quote!(#path BackgroundSize(#v)),
            Style::Border(v) => quote!(#path Border(#v)),
            Style::BorderBottom(v) => quote!(#path BorderBottom(#v)),
            Style::BorderBottomColor(v) => quote!(#path BorderBottomColor(#v)),
            Style::BorderBottomLeftRadius(v) => quote!(#path BorderBottomLeftRadius(#v)),
            Style::BorderBottomRightRadius(v) => quote!(#path BorderBottomRightRadius(#v)),
            Style::BorderBottomStyle(v) => quote!(#path BorderBottomStyle(#v)),
            Style::BorderBottomWidth(v) => quote!(#path BorderBottomWidth(#v)),
            Style::BorderCollapse(v) => quote!(#path BorderCollapse(#v)),
            Style::BorderColor(v) => quote!(#path BorderColor(#v)),
            // border-image
            // border-image-outset
            // border-image-repeat
            // border-image-slice
            // border-image-source
            // border-image-width
            Style::BorderLeft(v) => quote!(#path BorderLeft(#v)),
            Style::BorderLeftColor(v) => quote!(#path BorderLeftColor(#v)),
            Style::BorderLeftStyle(v) => quote!(#path BorderLeftStyle(#v)),
            Style::BorderLeftWidth(v) => quote!(#path BorderLeftWidth(#v)),
            Style::BorderRadius(v) => quote!(#path BorderRadius(#v)),
            Style::BorderRight(v) => quote!(#path BorderRight(#v)),
            Style::BorderRightColor(v) => quote!(#path BorderRightColor(#v)),
            Style::BorderRightStyle(v) => quote!(#path BorderRightStyle(#v)),
            Style::BorderRightWidth(v) => quote!(#path BorderRightWidth(#v)),
            // border-spacing
            Style::BorderStyle(v) => quote!(#path BorderStyle(#v)),
            Style::BorderTop(v) => quote!(#path BorderTop(#v)),
            Style::BorderTopColor(v) => quote!(#path BorderTopColor(#v)),
            Style::BorderTopLeftRadius(v) => quote!(#path BorderTopLeftRadius(#v)),
            Style::BorderTopRightRadius(v) => quote!(#path BorderTopRightRadius(#v)),
            Style::BorderTopStyle(v) => quote!(#path BorderTopStyle(#v)),
            Style::BorderTopWidth(v) => quote!(#path BorderTopWidth(#v)),
            Style::BorderWidth(v) => quote!(#path BorderWidth(#v)),
            Style::Bottom(v) => quote!(#path Bottom(#v)),
            // box-decoration-break
            Style::BoxShadow(v) => quote!(#path BoxShadow(#v)),
            Style::BoxSizing(v) => quote!(#path BoxSizing(#v)),
            // break-after
            // break-before
            // break-inside
            // caption-side
            // caret-color
            Style::Clear(v) => quote!(#path Clear(#v)),
            // clip
            // clip-path
            // clip-rule
            Style::ColumnCount(v) => quote!(#path ColumnCount(#v)),
            Style::Color(v) => quote!(#path Color(#v)),
            // contain
            // content
            // counter-increment
            // counter-reset
            // cue
            // cue-after
            // cue-before
            Style::Cursor(v) => quote!(#path Cursor(#v)),
            // direction
            Style::Display(v) => quote!(#path Display(#v)),
            // elevation
            // empty-cells
            // flex
            Style::FlexBasis(v) => quote!(#path FlexBasis(#v)),
            Style::FlexDirection(v) => quote!(#path FlexDirection(#v)),
            // flex-flow
            Style::FlexGrow(v) => quote!(#path FlexGrow(#v)),
            Style::FlexShrink(v) => quote!(#path FlexShrink(#v)),
            Style::FlexWrap(v) => quote!(#path FlexWrap(#v)),
            Style::Float(v) => quote!(#path Float(#v)),
            // font
            Style::FontFamily(v) => quote!(#path FontFamily(#v)),
            // font-feature-settings
            // font-kerning
            Style::FontSize(v) => quote!(#path FontSize(#v)),
            // font-size-adjust
            // font-stretch
            Style::FontStyle(v) => quote!(#path FontStyle(#v)),
            // font-synthesis
            // font-variant
            // font-variant-caps
            // font-variant-east-asian
            // font-variant-ligatures
            // font-variant-numeric
            // font-variant-position
            Style::FontWeight(v) => quote!(#path FontWeight(#v)),
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
            Style::Height(v) => quote!(#path Height(#v)),
            // image-orientation
            // image-rendering
            // isolation
            Style::JustifyContent(v) => quote!(#path JustifyContent(#v)),
            Style::Left(v) => quote!(#path Left(#v)),
            // letter-spacing
            Style::LineHeight(v) => quote!(#path LineHeight(#v)),
            // list-style
            // list-style-image
            // list-style-position
            Style::ListStyleType(v) => quote!(#path ListStyleType(#v)),
            Style::Margin(v) => quote!(#path Margin(#v)),
            Style::MarginBottom(v) => quote!(#path MarginBottom(#v)),
            Style::MarginLeft(v) => quote!(#path MarginLeft(#v)),
            Style::MarginRight(v) => quote!(#path MarginRight(#v)),
            Style::MarginTop(v) => quote!(#path MarginTop(#v)),
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
            Style::MaxHeight(v) => quote!(#path MaxHeight(#v)),
            Style::MaxWidth(v) => quote!(#path MaxWidth(#v)),
            Style::MinHeight(v) => quote!(#path MinHeight(#v)),
            Style::MinWidth(v) => quote!(#path MinWidth(#v)),
            // mix-blend-mode
            Style::ObjectFit(v) => quote!(#path ObjectFit(#v)),
            // object-position
            // opacity
            // order
            // orphans
            // outline
            // outline-color
            // outline-offset
            // outline-style
            // outline-width
            Style::Overflow(v) => quote!(#path Overflow(#v)),
            Style::OverflowX(v) => quote!(#path OverflowX(#v)),
            Style::OverflowY(v) => quote!(#path OverflowY(#v)),
            Style::Padding(v) => quote!(#path Padding(#v)),
            Style::PaddingBottom(v) => quote!(#path PaddingBottom(#v)),
            Style::PaddingLeft(v) => quote!(#path PaddingLeft(#v)),
            Style::PaddingRight(v) => quote!(#path PaddingRight(#v)),
            Style::PaddingTop(v) => quote!(#path PaddingTop(#v)),
            // page-break-after
            // page-break-before
            // page-break-inside
            // pause
            // pause-after
            // pause-before
            // pitch
            // pitch-range
            // play-during
            Style::Position(v) => quote!(#path Position(#v)),
            // quotes
            Style::Resize(v) => quote!(#path Resize(#v)),
            // richness
            Style::Right(v) => quote!(#path Right(#v)),
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
            Style::TextAlign(v) => quote!(#path TextAlign(#v)),
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
            Style::Top(v) => quote!(#path Top(#v)),
            // transform
            // transform-box
            // transform-origin
            // unicode-bidi
            // vertical-align
            // visibility
            // voice-family
            // volume
            // white-space
            Style::WhiteSpace(v) => quote!(#path WhiteSpace(#v)),
            Style::Widows(v) => quote!(#path Widows(#v)),
            Style::Width(v) => quote!(#path Width(#v)),
            // will-change
            // word-spacing
            // writing-mode
            // z-index
        });
    }
}

impl ToTokens for AlignContent {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(match self {
            AlignContent::FlexStart => path!(AlignContent::FlexStart),
            AlignContent::Center => path!(AlignContent::Center),
            AlignContent::FlexEnd => path!(style::AlignContent::FlexEnd),
            AlignContent::SpaceAround => path!(AlignContent::SpaceAround),
            AlignContent::SpaceBetween => path!(AlignContent::SpaceBetween),
            AlignContent::Stretch => path!(AlignContent::Stretch),
        });
    }
}

impl ToTokens for Cursor {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(match self {
            Cursor::Auto => path!(Cursor::Auto),
            Cursor::Default => path!(Cursor::Default),
            Cursor::None => path!(Cursor::None),
            Cursor::ContextMenu => path!(Cursor::ContextMenu),
            Cursor::Help => path!(Cursor::Help),
            Cursor::Pointer => path!(Cursor::Pointer),
            Cursor::Progress => path!(Cursor::Progress),
            Cursor::Wait => path!(Cursor::Wait),
            Cursor::Cell => path!(Cursor::Cell),
            Cursor::Crosshair => path!(Cursor::Crosshair),
            Cursor::Text => path!(Cursor::Text),
            Cursor::VerticalText => path!(Cursor::VerticalText),
            Cursor::Alias => path!(Cursor::Alias),
            Cursor::Copy => path!(Cursor::Copy),
            Cursor::Move => path!(Cursor::Move),
            Cursor::NoDrop => path!(Cursor::NoDrop),
            Cursor::NotAllowed => path!(Cursor::NotAllowed),
            Cursor::Grab => path!(Cursor::Grab),
            Cursor::Grabbing => path!(Cursor::Grabbing),
            Cursor::EResize => path!(Cursor::EResize),
            Cursor::NResize => path!(Cursor::NResize),
            Cursor::NEResize => path!(Cursor::NEResize),
            Cursor::NWResize => path!(Cursor::NWResize),
            Cursor::SResize => path!(Cursor::SResize),
            Cursor::SEResize => path!(Cursor::SEResize),
            Cursor::SWResize => path!(Cursor::SWResize),
            Cursor::WResize => path!(Cursor::WResize),
            Cursor::EWResize => path!(Cursor::EWResize),
            Cursor::NSResize => path!(Cursor::NSResize),
            Cursor::NESWResize => path!(Cursor::NESWResize),
            Cursor::NWSEResize => path!(Cursor::NWSEResize),
            Cursor::ColResize => path!(Cursor::ColResize),
            Cursor::RowResize => path!(Cursor::RowResize),
            Cursor::AllScroll => path!(Cursor::AllScroll),
            Cursor::ZoomIn => path!(Cursor::ZoomIn),
            Cursor::ZoomOut => path!(Cursor::ZoomOut),
        })
    }
}

impl ToTokens for Display {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(match self {
            Display::Block => path!(Display::Block),
            Display::Flex => path!(Display::Flex),
            Display::Inline => path!(Display::Inline),
        });
    }
}

impl ToTokens for FlexBasis {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(match self {
            FlexBasis::Width(v) => path!(FlexBasis::Width(#v)),
            FlexBasis::Content => path!(FlexBasis::Content),
        });
    }
}

impl ToTokens for FlexDirection {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(match self {
            FlexDirection::Row => path!(FlexDirection::Row),
            FlexDirection::Column => path!(FlexDirection::Column),
        });
    }
}

impl ToTokens for FlexWrap {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(match self {
            FlexWrap::Wrap => path!(FlexWrap::Wrap),
            FlexWrap::Nowrap => path!(FlexWrap::Nowrap),
        });
    }
}

impl ToTokens for AlignItems {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(match self {
            AlignItems::Normal => path!(AlignItems::Normal),
            AlignItems::Stretch => path!(AlignItems::Stretch),
            AlignItems::Center => path!(AlignItems::Center),
            AlignItems::Start => path!(AlignItems::Start),
            AlignItems::End => path!(AlignItems::End),
            AlignItems::FlexStart => path!(AlignItems::FlexStart),
            AlignItems::FlexEnd => path!(AlignItems::FlexEnd),
            AlignItems::Baseline => path!(AlignItems::Baseline),
            AlignItems::FirstBaseline => path!(AlignItems::FirstBaseline),
            AlignItems::LastBaseline => path!(AlignItems::LastBaseline),
            AlignItems::SafeCenter => path!(AlignItems::SafeCenter),
            AlignItems::UnsafeCenter => path!(AlignItems::UnsafeCenter),
        });
    }
}

impl ToTokens for AlignSelf {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(match self {
            AlignSelf::Auto => path!(AlignSelf::Auto),
            AlignSelf::Normal => path!(AlignSelf::Normal),
            AlignSelf::Center => path!(AlignSelf::Center),
            AlignSelf::Start => path!(AlignSelf::Start),
            AlignSelf::End => path!(AlignSelf::End),
            AlignSelf::SelfStart => path!(AlignSelf::SelfStart),
            AlignSelf::SelfEnd => path!(AlignSelf::SelfEnd),
            AlignSelf::FlexStart => path!(AlignSelf::FlexStart),
            AlignSelf::FlexEnd => path!(AlignSelf::FlexEnd),
            AlignSelf::Baseline => path!(AlignSelf::Baseline),
            AlignSelf::FirstBaseline => path!(AlignSelf::FirstBaseline),
            AlignSelf::LastBaseline => path!(AlignSelf::LastBaseline),
            AlignSelf::Stretch => path!(AlignSelf::Stretch),
            AlignSelf::SafeCenter => path!(AlignSelf::SafeCenter),
            AlignSelf::UnsafeCenter => path!(AlignSelf::UnsafeCenter),
        });
    }
}

impl ToTokens for BackgroundAttachment {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(match self {
            BackgroundAttachment::Scroll => path!(BackgroundAttachment::Scroll),
            BackgroundAttachment::Fixed => path!(BackgroundAttachment::Fixed),
            BackgroundAttachment::Local => path!(BackgroundAttachment::Local),
        })
    }
}

impl ToTokens for BlendMode {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(match self {
            BlendMode::Normal => path!(BlendMode::Normal),
            BlendMode::Multiply => path!(BlendMode::Multiply),
            BlendMode::Screen => path!(BlendMode::Screen),
            BlendMode::Overlay => path!(BlendMode::Overlay),
            BlendMode::Darken => path!(BlendMode::Darken),
            BlendMode::Lighten => path!(BlendMode::Lighten),
            BlendMode::ColorDodge => path!(BlendMode::ColorDodge),
            BlendMode::ColorBurn => path!(BlendMode::ColorBurn),
            BlendMode::HardLight => path!(BlendMode::HardLight),
            BlendMode::SoftLight => path!(BlendMode::SoftLight),
            BlendMode::Difference => path!(BlendMode::Difference),
            BlendMode::Exclusion => path!(BlendMode::Exclusion),
            BlendMode::Hue => path!(BlendMode::Hue),
            BlendMode::Saturation => path!(BlendMode::Saturation),
            BlendMode::Color => path!(BlendMode::Color),
            BlendMode::Luminosity => path!(BlendMode::Luminosity),
        })
    }
}

impl ToTokens for BackgroundBox {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(match self {
            BackgroundBox::BorderBox => path!(BackgroundBox::BorderBox),
            BackgroundBox::PaddingBox => path!(BackgroundBox::PaddingBox),
            BackgroundBox::ContentBox => path!(BackgroundBox::ContentBox),
        })
    }
}

impl ToTokens for BackgroundImage {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(match self {
            BackgroundImage::None => path!(BackgroundImage::None),
            BackgroundImage::Url(url) => path!(BackgroundImage::Url(#url)),
        })
    }
}

impl ToTokens for BackgroundPosition {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(match self {
            BackgroundPosition::Top => path!(BackgroundPosition::Top),
            BackgroundPosition::Bottom => path!(BackgroundPosition::Bottom),
            BackgroundPosition::Left => path!(BackgroundPosition::Left),
            BackgroundPosition::Right => path!(BackgroundPosition::Right),
            BackgroundPosition::Center => path!(BackgroundPosition::Center),
        })
    }
}

impl ToTokens for BackgroundRepeat {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(match self {
            BackgroundRepeat::RepeatX => path!(BackgroundRepeat::RepeatX),
            BackgroundRepeat::RepeatY => path!(BackgroundRepeat::RepeatY),
            BackgroundRepeat::SingleOrDouble(v) => path!(BackgroundRepeat::SingleOrDouble(#v)),
        })
    }
}

impl ToTokens for BgRepeatPart {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(match self {
            BgRepeatPart::Repeat => path!(BgRepeatPart::Repeat),
            BgRepeatPart::Space => path!(BgRepeatPart::Space),
            BgRepeatPart::Round => path!(BgRepeatPart::Round),
            BgRepeatPart::NoRepeat => path!(BgRepeatPart::NoRepeat),
        })
    }
}

impl ToTokens for BackgroundSize {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(match self {
            BackgroundSize::Cover => path!(BackgroundSize::Cover),
            BackgroundSize::Contain => path!(BackgroundSize::Contain),
            BackgroundSize::SingleOrDouble(v) => path!(BackgroundSize::SingleOrDouble(#v)),
        })
    }
}

impl ToTokens for BorderCollapse {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(match self {
            BorderCollapse::Collapse => path!(BorderCollapse::Collapse),
            BorderCollapse::Separate => path!(BorderCollapse::Separate),
        })
    }
}

impl ToTokens for JustifyContent {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(match self {
            JustifyContent::FlexStart => path!(JustifyContent::FlexStart),
            JustifyContent::Center => path!(JustifyContent::Center),
            JustifyContent::FlexEnd => path!(JustifyContent::FlexEnd),
            JustifyContent::SpaceAround => path!(JustifyContent::SpaceAround),
            JustifyContent::SpaceBetween => path!(JustifyContent::SpaceBetween),
        });
    }
}

impl ToTokens for Float {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(match self {
            Float::None => path!(Float::None),
            Float::Left => path!(Float::Left),
            Float::Right => path!(Float::Right),
            Float::InlineStart => path!(Float::InlineStart),
            Float::InlineEnd => path!(Float::InlineEnd),
        })
    }
}

impl ToTokens for FontWeight {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(match self {
            FontWeight::Normal => path!(FontWeight::Normal),
            FontWeight::Bold => path!(FontWeight::Bold),
            FontWeight::Lighter => path!(FontWeight::Lighter),
            FontWeight::Bolder => path!(FontWeight::Bolder),
            FontWeight::Number(v) => path!(FontWeight::Number(#v)),
        });
    }
}

impl ToTokens for Font {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(match self {
            Font::Named(inner) => path!(Font::Named(String::from(#inner))),
            Font::Serif => path!(Font::Serif),
            Font::SansSerif => path!(Font::SansSerif),
            Font::Cursive => path!(Font::Cursive),
            Font::Fantasy => path!(Font::Fantasy),
            Font::Monospace => path!(Font::Monospace),
        })
    }
}

impl ToTokens for FontSize {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(match self {
            FontSize::XXSmall => path!(FontSize::XXSmall),
            FontSize::XSmall => path!(FontSize::XSmall),
            FontSize::Small => path!(FontSize::Small),
            FontSize::Medium => path!(FontSize::Medium),
            FontSize::Large => path!(FontSize::Large),
            FontSize::XLarge => path!(FontSize::XLarge),
            FontSize::XXLarge => path!(FontSize::XXLarge),
            FontSize::XXXLarge => path!(FontSize::XXXLarge),
            FontSize::Larger => path!(FontSize::Larger),
            FontSize::Smaller => path!(FontSize::Smaller),
            FontSize::LengthPercentage(v) => path!(FontSize::LengthPercentage(#v)),
        });
    }
}

impl ToTokens for FontStyle {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(match self {
            FontStyle::Normal => path!(FontStyle::Normal),
            FontStyle::Italic => path!(FontStyle::Italic),
            FontStyle::Oblique => path!(FontStyle::Oblique),
        });
    }
}

impl ToTokens for Border {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let line_width = match self.line_width {
            Some(line_width) => quote!(Some(#line_width)),
            None => quote!(None),
        };
        let line_style = match self.line_style {
            Some(line_style) => quote!(Some(#line_style)),
            None => quote!(None),
        };
        let color = match self.color {
            Some(color) => quote!(Some(#color)),
            None => quote!(None),
        };
        tokens.extend(quote!(
            style::Border {
                line_width: #line_width,
                line_style: #line_style,
                color: #color,
            }
        ))
    }
}

impl ToTokens for BoxShadow {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(match self {
            BoxShadow::None => path!(BoxShadow::None),
            BoxShadow::Shadows(list) => path!(BoxShadow::Shadows(#list)),
        });
    }
}

impl ToTokens for BoxSizing {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(match self {
            BoxSizing::BorderBox => path!(BoxSizing::BorderBox),
            BoxSizing::ContentBox => path!(BoxSizing::ContentBox),
        });
    }
}

impl ToTokens for Clear {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(match self {
            Clear::None => path!(Clear::None),
            Clear::Left => path!(Clear::Left),
            Clear::Right => path!(Clear::Right),
            Clear::Both => path!(Clear::Both),
            Clear::InlineStart => path!(Clear::InlineStart),
            Clear::InlineEnd => path!(Clear::InlineEnd),
        })
    }
}

impl ToTokens for ColumnCount {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(match self {
            ColumnCount::Auto => path!(ColumnCount::Auto),
            ColumnCount::Fixed(v) => path!(ColumnCount::Fixed(#v)),
        })
    }
}

impl ToTokens for Overflow {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(match self {
            Overflow::Both(v) => path!(Overflow::Both(#v)),
            Overflow::XY(x, y) => path!(Overflow::XY(#x, #y)),
        })
    }
}

impl ToTokens for OverflowXY {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(match self {
            OverflowXY::Visible => path!(OverflowXY::Visible),
            OverflowXY::Hidden => path!(OverflowXY::Hidden),
            OverflowXY::Clip => path!(OverflowXY::Clip),
            OverflowXY::Scroll => path!(OverflowXY::Scroll),
            OverflowXY::Auto => path!(OverflowXY::Auto),
        })
    }
}

impl ToTokens for ObjectFit {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(match self {
            ObjectFit::Fill => path!(ObjectFit::Fill),
            ObjectFit::None => path!(ObjectFit::None),
            ObjectFit::Contain { scale_down } => {
                path!(ObjectFit::Contain { scale_down: #scale_down })
            }
            ObjectFit::Cover { scale_down } => path!(ObjectFit::Cover { scale_down: #scale_down }),
        })
    }
}

impl<T> ToTokens for Rect<T>
where
    T: ToTokens,
{
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(match self {
            Rect::All(v) => path!(Rect::All(#v)),
            Rect::VerticalHorizontal(v, h) => path!(Rect::VerticalHorizontal(#v, #h)),
            Rect::TopHorizontalBottom(t, h, b) => path!(Rect::TopHorizontalBottom(#t, #h, #b)),
            Rect::TopRightBottomLeft(t, r, b, l) => path!(Rect::TopRightBottomLeft(#t, #r, #b, #l)),
        });
    }
}

impl ToTokens for LengthPercentage {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(match self {
            LengthPercentage::Length(v) => path!(LengthPercentage::Length(#v)),
            LengthPercentage::Percentage(v) => path!(LengthPercentage::Percentage(#v)),
        });
    }
}

impl ToTokens for AutoLengthPercentage {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(match self {
            AutoLengthPercentage::LengthPercentage(v) => {
                path!(AutoLengthPercentage::LengthPercentage(#v))
            }
            AutoLengthPercentage::Auto => path!(AutoLengthPercentage::Auto),
        });
    }
}

impl ToTokens for LineStyle {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(match self {
            LineStyle::None => path!(LineStyle::None),
            LineStyle::Hidden => path!(LineStyle::Hidden),
            LineStyle::Dotted => path!(LineStyle::Dotted),
            LineStyle::Dashed => path!(LineStyle::Dashed),
            LineStyle::Solid => path!(LineStyle::Solid),
            LineStyle::Double => path!(LineStyle::Double),
            LineStyle::Groove => path!(LineStyle::Groove),
            LineStyle::Ridge => path!(LineStyle::Ridge),
            LineStyle::Inset => path!(LineStyle::Inset),
            LineStyle::Outset => path!(LineStyle::Outset),
        })
    }
}

impl ToTokens for LineWidth {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(match self {
            LineWidth::Length(length) => path!(LineWidth::Length(#length)),
            LineWidth::Thin => path!(LineWidth::Thin),
            LineWidth::Medium => path!(LineWidth::Medium),
            LineWidth::Thick => path!(LineWidth::Thick),
        })
    }
}

impl ToTokens for LineHeight {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.0.to_tokens(tokens)
    }
}

impl ToTokens for ListStyleType {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(match self {
            ListStyleType::Disc => path!(ListStyleType::Disc),
            ListStyleType::Circle => path!(ListStyleType::Circle),
            ListStyleType::Square => path!(ListStyleType::Square),
            ListStyleType::Decimal => path!(ListStyleType::Decimal),
            ListStyleType::DecimalLeadingZero => path!(ListStyleType::DecimalLeadingZero),
            ListStyleType::LowerRoman => path!(ListStyleType::LowerRoman),
            ListStyleType::UpperRoman => path!(ListStyleType::UpperRoman),
            ListStyleType::LowerGreek => path!(ListStyleType::LowerGreek),
            ListStyleType::UpperGreek => path!(ListStyleType::UpperGreek),
            ListStyleType::LowerLatin => path!(ListStyleType::LowerLatin),
            ListStyleType::UpperLatin => path!(ListStyleType::UpperLatin),
            ListStyleType::Armenian => path!(ListStyleType::Armenian),
            ListStyleType::Georgian => path!(ListStyleType::Georgian),
            ListStyleType::LowerAlpha => path!(ListStyleType::LowerAlpha),
            ListStyleType::UpperAlpha => path!(ListStyleType::UpperAlpha),
            ListStyleType::None => path!(ListStyleType::None),
        })
    }
}

impl ToTokens for Position {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(match self {
            Position::Static => path!(Position::Static),
            Position::Relative => path!(Position::Relative),
            Position::Absolute => path!(Position::Absolute),
            Position::Fixed => path!(Position::Fixed),
        })
    }
}

impl ToTokens for Resize {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(match self {
            Resize::None => path!(Resize::None),
            Resize::Both => path!(Resize::Both),
            Resize::Horizontal => path!(Resize::Horizontal),
            Resize::Vertical => path!(Resize::Vertical),
        })
    }
}

impl ToTokens for WhiteSpace {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(match self {
            WhiteSpace::Normal => path!(WhiteSpace::Normal),
            WhiteSpace::Pre => path!(WhiteSpace::Pre),
            WhiteSpace::Nowrap => path!(WhiteSpace::Nowrap),
            WhiteSpace::PreWrap => path!(WhiteSpace::PreWrap),
            WhiteSpace::PreLine => path!(WhiteSpace::PreLine),
        })
    }
}

impl ToTokens for WidthHeight {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(match self {
            WidthHeight::Auto => path!(WidthHeight::Auto),
            WidthHeight::LengthPercentage(v) => path!(WidthHeight::LengthPercentage(#v)),
            WidthHeight::MinContent => path!(WidthHeight::MinContent),
            WidthHeight::MaxContent => path!(WidthHeight::MaxContent),
            WidthHeight::FitContent(v) => path!(WidthHeight::FitContent(#v)),
        })
    }
}

impl ToTokens for MaxWidthHeight {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(match self {
            MaxWidthHeight::None => path!(MaxWidthHeight::None),
            MaxWidthHeight::LengthPercentage(v) => path!(MaxWidthHeight::LengthPercentage(#v)),
            MaxWidthHeight::MinContent => path!(MaxWidthHeight::MinContent),
            MaxWidthHeight::MaxContent => path!(MaxWidthHeight::MaxContent),
            MaxWidthHeight::FitContent(v) => path!(MaxWidthHeight::FitContent(#v)),
        })
    }
}

impl ToTokens for Width21 {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(match self {
            Width21::Auto => path!(Width21::Auto),
            Width21::LengthPercentage(v) => path!(Width21::LengthPercentage(#v)),
        })
    }
}

impl ToTokens for Shadow {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let color = match self.color.as_ref() {
            Some(color) => quote!(Some(#color)),
            None => quote!(None),
        };
        let length = &self.length;
        let inset = &self.inset;
        tokens.extend(quote! {
            style::Shadow {
                color: #color,
                length: #length,
                inset: #inset,
            }
        })
    }
}

impl ToTokens for ShadowLength {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(match self {
            ShadowLength::Offsets {
                vertical,
                horizontal,
            } => path!(ShadowLength::Offsets {
                vertical: #vertical,
                horizontal: #horizontal,
            }),
            ShadowLength::OffsetsBlur {
                vertical,
                horizontal,
                blur,
            } => path!(ShadowLength::OffsetsBlur {
                vertical: #vertical,
                horizontal: #horizontal,
                blur: #blur,
            }),
            ShadowLength::OffsetsBlurSpread {
                vertical,
                horizontal,
                blur,
                spread,
            } => path!(ShadowLength::Offsets {
                vertical: #vertical,
                horizontal: #horizontal,
                blur: #blur,
                spread: #spread,
            }),
        })
    }
}

impl ToTokens for TextAlign {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(match self {
            TextAlign::Left => path!(TextAlign::Left),
            TextAlign::Right => path!(TextAlign::Right),
            TextAlign::Center => path!(TextAlign::Center),
            TextAlign::Justify => path!(TextAlign::Justify),
        });
    }
}

impl ToTokens for Length {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(match self {
            Length::Em(v) => path!(Length::Em(#v)),
            Length::Ex(v) => path!(Length::Ex(#v)),
            Length::In(v) => path!(Length::In(#v)),
            Length::Cm(v) => path!(Length::Cm(#v)),
            Length::Mm(v) => path!(Length::Mm(#v)),
            Length::Pt(v) => path!(Length::Pt(#v)),
            Length::Pc(v) => path!(Length::Pc(#v)),
            Length::Px(v) => path!(Length::Px(#v)),
            Length::Zero => path!(Length::Zero),
        })
    }
}

impl ToTokens for Percentage {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let val = self.0;
        tokens.extend(path!(Percentage(#val)));
    }
}

impl ToTokens for DynamicColor {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(match self {
            DynamicColor::Dynamic(block) => path!(DynamicColor::Literal(#block)),
            DynamicColor::Literal(color) => path!(DynamicColor::Literal(#color)),
        })
    }
}

impl ToTokens for Color {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(match self {
            Color::HexRGB(r, g, b) => path!(Color::HexRGB(#r, #g, #b)),
            Color::HexRGBA(r, g, b, a) => path!(Color::HexRGB(#r, #g, #b, #a)),
            Color::HSL(h, s, l) => path!(Color::HSL(#h, #s, #l)),
            Color::HSLA(h, s, l, a) => path!(Color::HSLA(#h, #s, #l, #a)),
            Color::IndianRed => path!(Color::IndianRed),
            Color::LightCoral => path!(Color::LightCoral),
            Color::Salmon => path!(Color::Salmon),
            Color::DarkSalmon => path!(Color::DarkSalmon),
            Color::LightSalmon => path!(Color::LightSalmon),
            Color::Crimson => path!(Color::Crimson),
            Color::Red => path!(Color::Red),
            Color::FireBrick => path!(Color::FireBrick),
            Color::DarkRed => path!(Color::DarkRed),
            Color::Pink => path!(Color::Pink),
            Color::LightPink => path!(Color::LightPink),
            Color::HotPink => path!(Color::HotPink),
            Color::DeepPink => path!(Color::DeepPink),
            Color::MediumVioletRed => path!(Color::MediumVioletRed),
            Color::PaleVioletRed => path!(Color::PaleVioletRed),
            Color::Coral => path!(Color::Coral),
            Color::Tomato => path!(Color::Tomato),
            Color::OrangeRed => path!(Color::OrangeRed),
            Color::DarkOrange => path!(Color::DarkOrange),
            Color::Orange => path!(Color::Orange),
            Color::Gold => path!(Color::Gold),
            Color::Yellow => path!(Color::Yellow),
            Color::LightYellow => path!(Color::LightYellow),
            Color::LemonChiffon => path!(Color::LemonChiffon),
            Color::LightGoldenrodYellow => path!(Color::LightGoldenrodYellow),
            Color::PapayaWhip => path!(Color::PapayaWhip),
            Color::Moccasin => path!(Color::Moccasin),
            Color::PeachPuff => path!(Color::PeachPuff),
            Color::PaleGoldenrod => path!(Color::PaleGoldenrod),
            Color::Khaki => path!(Color::Khaki),
            Color::DarkKhaki => path!(Color::DarkKhaki),
            Color::Lavender => path!(Color::Lavender),
            Color::Thistle => path!(Color::Thistle),
            Color::Plum => path!(Color::Plum),
            Color::Violet => path!(Color::Violet),
            Color::Orchid => path!(Color::Orchid),
            Color::Fuchsia => path!(Color::Fuchsia),
            Color::Magenta => path!(Color::Magenta),
            Color::MediumOrchid => path!(Color::MediumOrchid),
            Color::MediumPurple => path!(Color::MediumPurple),
            Color::RebeccaPurple => path!(Color::RebeccaPurple),
            Color::BlueViolet => path!(Color::BlueViolet),
            Color::DarkViolet => path!(Color::DarkViolet),
            Color::DarkOrchid => path!(Color::DarkOrchid),
            Color::DarkMagenta => path!(Color::DarkMagenta),
            Color::Purple => path!(Color::Purple),
            Color::Indigo => path!(Color::Indigo),
            Color::SlateBlue => path!(Color::SlateBlue),
            Color::DarkSlateBlue => path!(Color::DarkSlateBlue),
            Color::MediumSlateBlue => path!(Color::MediumSlateBlue),
            Color::GreenYellow => path!(Color::GreenYellow),
            Color::Chartreuse => path!(Color::Chartreuse),
            Color::LawnGreen => path!(Color::LawnGreen),
            Color::Lime => path!(Color::Lime),
            Color::LimeGreen => path!(Color::LimeGreen),
            Color::PaleGreen => path!(Color::PaleGreen),
            Color::LightGreen => path!(Color::LightGreen),
            Color::MediumSpringGreen => path!(Color::MediumSpringGreen),
            Color::SpringGreen => path!(Color::SpringGreen),
            Color::MediumSeaGreen => path!(Color::MediumSeaGreen),
            Color::SeaGreen => path!(Color::SeaGreen),
            Color::ForestGreen => path!(Color::ForestGreen),
            Color::Green => path!(Color::Green),
            Color::DarkGreen => path!(Color::DarkGreen),
            Color::YellowGreen => path!(Color::YellowGreen),
            Color::OliveDrab => path!(Color::OliveDrab),
            Color::Olive => path!(Color::Olive),
            Color::DarkOliveGreen => path!(Color::DarkOliveGreen),
            Color::MediumAquamarine => path!(Color::MediumAquamarine),
            Color::DarkSeaGreen => path!(Color::DarkSeaGreen),
            Color::LightSeaGreen => path!(Color::LightSeaGreen),
            Color::DarkCyan => path!(Color::DarkCyan),
            Color::Teal => path!(Color::Teal),
            Color::Aqua => path!(Color::Aqua),
            Color::Cyan => path!(Color::Cyan),
            Color::LightCyan => path!(Color::LightCyan),
            Color::PaleTurquoise => path!(Color::PaleTurquoise),
            Color::Aquamarine => path!(Color::Aquamarine),
            Color::Turquoise => path!(Color::Turquoise),
            Color::MediumTurquoise => path!(Color::MediumTurquoise),
            Color::DarkTurquoise => path!(Color::DarkTurquoise),
            Color::CadetBlue => path!(Color::CadetBlue),
            Color::SteelBlue => path!(Color::SteelBlue),
            Color::LightSteelBlue => path!(Color::LightSteelBlue),
            Color::PowderBlue => path!(Color::PowderBlue),
            Color::LightBlue => path!(Color::LightBlue),
            Color::SkyBlue => path!(Color::SkyBlue),
            Color::LightSkyBlue => path!(Color::LightSkyBlue),
            Color::DeepSkyBlue => path!(Color::DeepSkyBlue),
            Color::DodgerBlue => path!(Color::DodgerBlue),
            Color::CornflowerBlue => path!(Color::CornflowerBlue),
            Color::RoyalBlue => path!(Color::RoyalBlue),
            Color::Blue => path!(Color::Blue),
            Color::MediumBlue => path!(Color::MediumBlue),
            Color::DarkBlue => path!(Color::DarkBlue),
            Color::Navy => path!(Color::Navy),
            Color::MidnightBlue => path!(Color::MidnightBlue),
            Color::Cornsilk => path!(Color::Cornsilk),
            Color::BlanchedAlmond => path!(Color::BlanchedAlmond),
            Color::Bisque => path!(Color::Bisque),
            Color::NavajoWhite => path!(Color::NavajoWhite),
            Color::Wheat => path!(Color::Wheat),
            Color::BurlyWood => path!(Color::BurlyWood),
            Color::Tan => path!(Color::Tan),
            Color::RosyBrown => path!(Color::RosyBrown),
            Color::SandyBrown => path!(Color::SandyBrown),
            Color::Goldenrod => path!(Color::Goldenrod),
            Color::DarkGoldenrod => path!(Color::DarkGoldenrod),
            Color::Peru => path!(Color::Peru),
            Color::Chocolate => path!(Color::Chocolate),
            Color::SaddleBrown => path!(Color::SaddleBrown),
            Color::Sienna => path!(Color::Sienna),
            Color::Brown => path!(Color::Brown),
            Color::Maroon => path!(Color::Maroon),
            Color::White => path!(Color::White),
            Color::Snow => path!(Color::Snow),
            Color::HoneyDew => path!(Color::HoneyDew),
            Color::MintCream => path!(Color::MintCream),
            Color::Azure => path!(Color::Azure),
            Color::AliceBlue => path!(Color::AliceBlue),
            Color::GhostWhite => path!(Color::GhostWhite),
            Color::WhiteSmoke => path!(Color::WhiteSmoke),
            Color::SeaShell => path!(Color::SeaShell),
            Color::Beige => path!(Color::Beige),
            Color::OldLace => path!(Color::OldLace),
            Color::FloralWhite => path!(Color::FloralWhite),
            Color::Ivory => path!(Color::Ivory),
            Color::AntiqueWhite => path!(Color::AntiqueWhite),
            Color::Linen => path!(Color::Linen),
            Color::LavenderBlush => path!(Color::LavenderBlush),
            Color::MistyRose => path!(Color::MistyRose),
            Color::Gainsboro => path!(Color::Gainsboro),
            Color::LightGray => path!(Color::LightGray),
            Color::Silver => path!(Color::Silver),
            Color::DarkGray => path!(Color::DarkGray),
            Color::Gray => path!(Color::Gray),
            Color::DimGray => path!(Color::DimGray),
            Color::LightSlateGray => path!(Color::LightSlateGray),
            Color::SlateGray => path!(Color::SlateGray),
            Color::DarkSlateGray => path!(Color::DarkSlateGray),
            Color::Black => path!(Color::Black),
        })
    }
}

// Generic containers

impl<T> ToTokens for NonemptyCommaList<T>
where
    T: ToTokens,
{
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let first = &self.first;
        let rest = &self.rest;
        tokens.extend(path! {
            NonemptyCommaList {
                first: #first,
                rest: vec![#(#rest),*],
            }
        })
    }
}

impl<T> ToTokens for SingleOrDouble<T>
where
    T: ToTokens,
{
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(match self {
            SingleOrDouble::Single(t) => path!(SingleOrDouble::Single(#t)),
            SingleOrDouble::Double { vert, horiz } => path!(SingleOrDouble::Double {
                vert: #vert,
                horiz: #horiz,
            }),
        })
    }
}
