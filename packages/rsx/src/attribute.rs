//! Parser for the attribute shared both by elements and components
//!
//! ```rust, ignore
//! rsx! {
//!     div {
//!         class: "my-class",
//!         onclick: move |_| println!("clicked")
//!     }
//!
//!     Component {
//!         class: "my-class",
//!         onclick: move |_| println!("clicked")
//!     }
//! }
//! ```

use super::literal::HotLiteral;
use crate::{innerlude::*, partial_closure::PartialClosure};

use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{ToTokens, TokenStreamExt, quote, quote_spanned};
use std::fmt::Display;
use syn::{
    Block, Expr, ExprBlock, ExprClosure, ExprIf, Ident, Lit, LitBool, LitFloat, LitInt, LitStr,
    Stmt, Token,
    ext::IdentExt,
    parse::{Parse, ParseStream},
    parse_quote,
    spanned::Spanned,
};

/// A property value in the from of a `name: value` pair with an optional comma.
/// Note that the colon and value are optional in the case of shorthand attributes. We keep them around
/// to support "lossless" parsing in case that ever might be useful.
#[derive(PartialEq, Eq, Clone, Debug, Hash)]
pub struct Attribute {
    /// The name of the attribute (ident or custom)
    ///
    /// IE `class` or `onclick`
    pub name: AttributeName,

    /// The colon that separates the name and value - keep this for lossless parsing
    pub colon: Option<Token![:]>,

    /// The value of the attribute
    ///
    /// IE `class="my-class"` or `onclick: move |_| println!("clicked")`
    pub value: AttributeValue,

    /// The comma that separates this attribute from the next one
    /// Used for more accurate completions
    pub comma: Option<Token![,]>,

    /// The element name of this attribute if it is bound to an element.
    /// When parsed for components or freestanding, this will be None
    pub el_name: Option<ElementName>,
}

impl Parse for Attribute {
    fn parse(content: ParseStream) -> syn::Result<Self> {
        // if there's an ident not followed by a colon, it's a shorthand attribute
        if content.peek(Ident::peek_any) && !content.peek2(Token![:]) {
            let ident = parse_raw_ident(content)?;
            let comma = content.parse().ok();

            return Ok(Attribute {
                name: AttributeName::BuiltIn(ident.clone()),
                colon: None,
                value: AttributeValue::Shorthand(ident),
                comma,
                el_name: None,
            });
        }

        // Parse the name as either a known or custom attribute
        let name = match content.peek(LitStr) {
            true => AttributeName::Custom(content.parse::<LitStr>()?),
            false => AttributeName::BuiltIn(parse_raw_ident(content)?),
        };

        // Ensure there's a colon
        let colon = Some(content.parse::<Token![:]>()?);

        // todo: make this cleaner please
        // if statements in attributes get automatic closing in some cases
        // we shouldn't be handling it any differently.
        let value = AttributeValue::parse(content)?;

        let comma = content.parse::<Token![,]>().ok();

        let attr = Attribute {
            name,
            value,
            colon,
            comma,
            el_name: None,
        };

        Ok(attr)
    }
}

impl Attribute {
    /// Create a new attribute from a name and value
    pub fn from_raw(name: AttributeName, value: AttributeValue) -> Self {
        Self {
            name,
            colon: Default::default(),
            value,
            comma: Default::default(),
            el_name: None,
        }
    }

    pub fn span(&self) -> proc_macro2::Span {
        self.name.span()
    }

    pub fn as_lit(&self) -> Option<&HotLiteral> {
        match &self.value {
            AttributeValue::AttrLiteral(lit) => Some(lit),
            _ => None,
        }
    }

    /// Run this closure against the attribute if it's hotreloadable
    pub fn with_literal(&self, f: impl FnOnce(&HotLiteral)) {
        if let AttributeValue::AttrLiteral(ifmt) = &self.value {
            f(ifmt);
        }
    }

    pub fn ifmt(&self) -> Option<&IfmtInput> {
        match &self.value {
            AttributeValue::AttrLiteral(HotLiteral::Fmted(input)) => Some(input),
            _ => None,
        }
    }

    pub fn as_static_str_literal(&self) -> Option<(&AttributeName, &IfmtInput)> {
        match &self.value {
            AttributeValue::AttrLiteral(lit) => match &lit {
                HotLiteral::Fmted(input) if input.is_static() => Some((&self.name, input)),
                _ => None,
            },
            _ => None,
        }
    }

    pub fn is_static_str_literal(&self) -> bool {
        self.as_static_str_literal().is_some()
    }

    pub fn rendered_as_dynamic_attr(&self) -> TokenStream2 {
        // Shortcut out with spreads
        if let AttributeName::Spread(_) = self.name {
            let AttributeValue::AttrExpr(expr) = &self.value else {
                unreachable!("Spread attributes should always be expressions")
            };
            return quote_spanned! { expr.span() => {#expr}.into_boxed_slice() };
        }

        let el_name = self
            .el_name
            .as_ref()
            .expect("el_name rendered as a dynamic attribute should always have an el_name set");

        let attribute = |name: &AttributeName| match name {
            AttributeName::BuiltIn(_) | AttributeName::Custom(_) => {
                let resolved = name.resolved(el_name);
                let name = resolved.name;
                quote! { #name }
            }
            AttributeName::Spread(_) => unreachable!("Spread attributes are handled elsewhere"),
        };

        let ns = |name: &AttributeName| {
            let namespace = name.resolved(el_name).namespace;
            match namespace {
                Some(namespace) => quote! { Some(#namespace) },
                None => quote! { None },
            }
        };

        let volatile = |name: &AttributeName| {
            let volatile = name.resolved(el_name).volatile;
            quote! { #volatile }
        };

        let attribute = {
            let value = &self.value;
            let name = &self.name;
            let is_not_event = !self.name.is_likely_event();

            match &self.value {
                AttributeValue::AttrLiteral(_)
                | AttributeValue::AttrExpr(_)
                | AttributeValue::Shorthand(_)
                | AttributeValue::IfExpr { .. }
                    if is_not_event =>
                {
                    let name = &self.name;
                    let ns = ns(name);
                    let volatile = volatile(name);
                    let attribute = attribute(name);
                    let value = quote! { #value };

                    quote! {
                        dioxus_core::Attribute::new(
                            #attribute,
                            #value,
                            #ns,
                            #volatile
                        )
                    }
                }
                AttributeValue::EventTokens(_) | AttributeValue::AttrExpr(_) => {
                    let (tokens, span) = match &self.value {
                        AttributeValue::EventTokens(tokens) => {
                            (tokens.to_token_stream(), tokens.span())
                        }
                        AttributeValue::AttrExpr(tokens) => {
                            (tokens.to_token_stream(), tokens.span())
                        }
                        _ => unreachable!(),
                    };

                    fn check_tokens_is_closure(tokens: &TokenStream2) -> bool {
                        if syn::parse2::<ExprClosure>(tokens.to_token_stream()).is_ok() {
                            return true;
                        }
                        let Ok(block) = syn::parse2::<ExprBlock>(tokens.to_token_stream()) else {
                            return false;
                        };
                        let mut block = &block;
                        loop {
                            match block.block.stmts.last() {
                                Some(Stmt::Expr(Expr::Closure(_), _)) => return true,
                                Some(Stmt::Expr(Expr::Block(b), _)) => {
                                    block = b;
                                    continue;
                                }
                                _ => return false,
                            }
                        }
                    }
                    match &self.name {
                        AttributeName::BuiltIn(name) => {
                            let event_tokens_is_closure = check_tokens_is_closure(&tokens);
                            let function_name =
                                quote_spanned! { span => dioxus_elements::events::#name };
                            let function = if event_tokens_is_closure {
                                // If we see an explicit closure, we can call the `call_with_explicit_closure` version of the event for better type inference
                                quote_spanned! { span => #function_name::call_with_explicit_closure }
                            } else {
                                function_name
                            };
                            quote_spanned! { span =>
                                #function(#tokens)
                            }
                        }
                        AttributeName::Custom(_) => unreachable!("Handled elsewhere in the macro"),
                        AttributeName::Spread(_) => unreachable!("Handled elsewhere in the macro"),
                    }
                }
                _ => {
                    quote_spanned! { value.span() => dioxus_elements::events::#name(#value) }
                }
            }
        };

        let attr_span = attribute.span();
        quote_spanned! { attr_span =>
            Box::new([
                {
                    #attribute
                }
            ])
        }
        .to_token_stream()
    }

    pub fn can_be_shorthand(&self) -> bool {
        // If it's a shorthand...
        if matches!(self.value, AttributeValue::Shorthand(_)) {
            return true;
        }

        // Or if it is a builtin attribute with a single ident value
        if let (AttributeName::BuiltIn(name), AttributeValue::AttrExpr(expr)) =
            (&self.name, &self.value)
            && let Ok(Expr::Path(path)) = expr.as_expr()
            && path.path.get_ident() == Some(name)
        {
            return true;
        }

        false
    }
}

#[derive(PartialEq, Eq, Clone, Debug, Hash)]
pub enum AttributeName {
    Spread(Token![..]),

    /// an attribute in the form of `name: value`
    BuiltIn(Ident),

    /// an attribute in the form of `"name": value` - notice that the name is a string literal
    /// this is to allow custom attributes in the case of missing built-in attributes
    ///
    /// we might want to change this one day to be ticked or something and simply a boolean
    Custom(LitStr),
}

impl AttributeName {
    pub(crate) fn resolved(&self, element: &ElementName) -> ResolvedAttribute {
        match self {
            Self::BuiltIn(ident) => resolve_builtin_attribute(element, ident),
            Self::Custom(lit) => ResolvedAttribute {
                name: lit.value(),
                namespace: None,
                volatile: false,
            },
            Self::Spread(_) => unreachable!("spread attributes do not have static metadata"),
        }
    }

    pub fn is_likely_event(&self) -> bool {
        matches!(self, Self::BuiltIn(ident) if ident.to_string().starts_with("on"))
    }

    pub fn is_likely_key(&self) -> bool {
        matches!(self, Self::BuiltIn(ident) if ident == "key")
    }

    pub fn span(&self) -> proc_macro2::Span {
        match self {
            Self::Custom(lit) => lit.span(),
            Self::BuiltIn(ident) => ident.span(),
            Self::Spread(dots) => dots.span(),
        }
    }
}

pub(crate) struct ResolvedAttribute {
    pub(crate) name: String,
    pub(crate) namespace: Option<&'static str>,
    pub(crate) volatile: bool,
}

fn resolve_builtin_attribute(element: &ElementName, ident: &Ident) -> ResolvedAttribute {
    let element_name = element.tag_name_string();
    let raw_name = ident.to_string();
    let ident_name = raw_name.strip_prefix("r#").unwrap_or(&raw_name);
    let name = resolve_attribute_name(&element_name, ident_name);
    let namespace = resolve_attribute_namespace(element, ident_name, &name);
    let volatile = matches!(
        (element_name.as_str(), ident_name),
        ("input", "value") | ("option", "selected") | ("select", "value") | ("textarea", "value")
    );

    ResolvedAttribute {
        name,
        namespace,
        volatile,
    }
}

fn resolve_attribute_name(element: &str, ident: &str) -> String {
    match (element, ident) {
        (_, "as") => "as".to_string(),
        (_, "async") => "async".to_string(),
        (_, "for") => "for".to_string(),
        (_, "loop") => "loop".to_string(),
        (_, "type") => "type".to_string(),
        ("meta", "http_equiv") => "http-equiv".to_string(),
        ("iframe", "margin_width") => "marginWidth".to_string(),
        ("iframe", "margin_height") => "marginHeight".to_string(),
        ("iframe", "frame_border") => "frameBorder".to_string(),
        ("input", "directory") => "webkitdirectory".to_string(),
        ("annotation-xml", "encoding") => "encoding".to_string(),
        (_, "webkit_user_select") => "-webkit-user-select".to_string(),
        _ if ident.starts_with("aria_") => ident.replace('_', "-"),
        _ if is_style_attribute(ident) => ident.replace('_', "-"),
        _ => ident.to_string(),
    }
}

fn resolve_attribute_namespace(
    element: &ElementName,
    ident: &str,
    resolved_name: &str,
) -> Option<&'static str> {
    if element.namespace().is_some() {
        return None;
    }

    if is_style_attribute(ident) && !matches!(resolved_name, "style" | "class" | "id") {
        Some("style")
    } else {
        None
    }
}

fn is_style_attribute(ident: &str) -> bool {
    matches!(
        ident,
        "align_content"
            | "align_items"
            | "align_self"
            | "alignment_adjust"
            | "alignment_baseline"
            | "all"
            | "alt"
            | "animation"
            | "animation_delay"
            | "animation_direction"
            | "animation_duration"
            | "animation_fill_mode"
            | "animation_iteration_count"
            | "animation_name"
            | "animation_play_state"
            | "animation_timing_function"
            | "aspect_ratio"
            | "azimuth"
            | "backdrop_filter"
            | "backface_visibility"
            | "background"
            | "background_attachment"
            | "background_blend_mode"
            | "background_clip"
            | "background_color"
            | "background_image"
            | "background_origin"
            | "background_position"
            | "background_repeat"
            | "background_size"
            | "baseline_shift"
            | "border"
            | "border_bottom"
            | "border_bottom_color"
            | "border_bottom_left_radius"
            | "border_bottom_right_radius"
            | "border_bottom_style"
            | "border_bottom_width"
            | "border_collapse"
            | "border_color"
            | "border_left"
            | "border_left_color"
            | "border_left_style"
            | "border_left_width"
            | "border_radius"
            | "border_right"
            | "border_right_color"
            | "border_right_style"
            | "border_right_width"
            | "border_spacing"
            | "border_style"
            | "border_top"
            | "border_top_color"
            | "border_top_left_radius"
            | "border_top_right_radius"
            | "border_top_style"
            | "border_top_width"
            | "border_width"
            | "bottom"
            | "box_shadow"
            | "box_sizing"
            | "caption_side"
            | "clear"
            | "clip"
            | "clip_path"
            | "clip_rule"
            | "color"
            | "column_count"
            | "column_gap"
            | "columns"
            | "content"
            | "cursor"
            | "direction"
            | "display"
            | "dominant_baseline"
            | "fill"
            | "fill_opacity"
            | "fill_rule"
            | "filter"
            | "flex"
            | "flex_basis"
            | "flex_direction"
            | "flex_flow"
            | "flex_grow"
            | "flex_shrink"
            | "flex_wrap"
            | "float"
            | "font"
            | "font_family"
            | "font_size"
            | "font_style"
            | "font_weight"
            | "gap"
            | "grid"
            | "grid_area"
            | "grid_auto_columns"
            | "grid_auto_flow"
            | "grid_auto_rows"
            | "grid_column"
            | "grid_column_end"
            | "grid_column_start"
            | "grid_row"
            | "grid_row_end"
            | "grid_row_start"
            | "grid_template"
            | "grid_template_areas"
            | "grid_template_columns"
            | "grid_template_rows"
            | "height"
            | "hyphens"
            | "image_rendering"
            | "isolation"
            | "justify_content"
            | "justify_items"
            | "justify_self"
            | "left"
            | "letter_spacing"
            | "line_height"
            | "list_style"
            | "list_style_image"
            | "list_style_position"
            | "list_style_type"
            | "margin"
            | "margin_bottom"
            | "margin_left"
            | "margin_right"
            | "margin_top"
            | "marker"
            | "marker_end"
            | "marker_mid"
            | "marker_start"
            | "mask"
            | "mask_image"
            | "max_height"
            | "max_width"
            | "min_height"
            | "min_width"
            | "object_fit"
            | "object_position"
            | "opacity"
            | "order"
            | "outline"
            | "outline_color"
            | "outline_offset"
            | "outline_style"
            | "outline_width"
            | "overflow"
            | "overflow_x"
            | "overflow_y"
            | "padding"
            | "padding_bottom"
            | "padding_left"
            | "padding_right"
            | "padding_top"
            | "perspective"
            | "pointer_events"
            | "position"
            | "resize"
            | "right"
            | "row_gap"
            | "scroll_behavior"
            | "shape_rendering"
            | "size"
            | "stop_color"
            | "stop_opacity"
            | "stroke"
            | "stroke_dasharray"
            | "stroke_dashoffset"
            | "stroke_linecap"
            | "stroke_linejoin"
            | "stroke_miterlimit"
            | "stroke_opacity"
            | "stroke_width"
            | "text_align"
            | "text_anchor"
            | "text_decoration"
            | "text_overflow"
            | "text_rendering"
            | "text_shadow"
            | "top"
            | "touch_action"
            | "transform"
            | "transform_origin"
            | "transition"
            | "transition_delay"
            | "transition_duration"
            | "transition_property"
            | "transition_timing_function"
            | "user_select"
            | "webkit_user_select"
            | "vertical_align"
            | "visibility"
            | "white_space"
            | "width"
            | "will_change"
            | "word_break"
            | "word_spacing"
            | "word_wrap"
            | "writing_mode"
            | "z_index"
    )
}

impl Display for AttributeName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Custom(lit) => write!(f, "{}", lit.value()),
            Self::BuiltIn(ident) => write!(f, "{}", ident),
            Self::Spread(_) => write!(f, ".."),
        }
    }
}

impl ToTokens for AttributeName {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        match self {
            Self::Custom(lit) => lit.to_tokens(tokens),
            Self::BuiltIn(ident) => ident.to_tokens(tokens),
            Self::Spread(dots) => dots.to_tokens(tokens),
        }
    }
}

// ..spread attribute
#[derive(PartialEq, Eq, Clone, Debug, Hash)]
pub struct Spread {
    pub dots: Token![..],
    pub expr: Expr,
    pub comma: Option<Token![,]>,
}

impl Spread {
    pub fn span(&self) -> proc_macro2::Span {
        self.dots.span()
    }
}

#[derive(PartialEq, Eq, Clone, Debug, Hash)]
pub enum AttributeValue {
    /// Just a regular shorthand attribute - an ident. Makes our parsing a bit more opaque.
    /// attribute,
    Shorthand(Ident),

    /// Any attribute that's a literal. These get hotreloading super powers
    ///
    /// attribute: "value"
    /// attribute: bool,
    /// attribute: 1,
    AttrLiteral(HotLiteral),

    /// A series of tokens that represent an event handler
    ///
    /// We use a special type here so we can get autocomplete in the closure using partial expansion.
    /// We also do some extra wrapping for improved type hinting since rust sometimes has trouble with
    /// generics and closures.
    EventTokens(PartialClosure),

    /// Conditional expression
    ///
    /// attribute: if bool { "value" } else if bool { "other value" } else { "default value" }
    ///
    /// Currently these don't get hotreloading super powers, but they could, depending on how far
    /// we want to go with it
    IfExpr(IfAttributeValue),

    /// attribute: some_expr
    /// attribute: {some_expr} ?
    AttrExpr(PartialExpr),
}

impl Parse for AttributeValue {
    fn parse(content: ParseStream) -> syn::Result<Self> {
        // Attempt to parse the unterminated if statement
        if content.peek(Token![if]) {
            return Ok(Self::IfExpr(content.parse::<IfAttributeValue>()?));
        }

        // Use the move and/or bars as an indicator that we have an event handler
        if content.peek(Token![move]) || content.peek(Token![|]) {
            let value = content.parse()?;
            return Ok(AttributeValue::EventTokens(value));
        }

        if content.peek(LitStr)
            || content.peek(LitBool)
            || content.peek(LitFloat)
            || content.peek(LitInt)
        {
            let fork = content.fork();
            _ = fork.parse::<Lit>().unwrap();

            if content.peek2(Token![,]) || fork.is_empty() {
                let value = content.parse()?;
                return Ok(AttributeValue::AttrLiteral(value));
            }
        }

        let value = content.parse::<PartialExpr>()?;
        Ok(AttributeValue::AttrExpr(value))
    }
}

impl ToTokens for AttributeValue {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match self {
            Self::Shorthand(ident) => ident.to_tokens(tokens),
            Self::AttrLiteral(ifmt) => ifmt.to_tokens(tokens),
            Self::IfExpr(if_expr) => if_expr.to_tokens(tokens),
            Self::AttrExpr(expr) => expr.to_tokens(tokens),
            Self::EventTokens(closure) => closure.to_tokens(tokens),
        }
    }
}

impl AttributeValue {
    pub fn span(&self) -> proc_macro2::Span {
        match self {
            Self::Shorthand(ident) => ident.span(),
            Self::AttrLiteral(ifmt) => ifmt.span(),
            Self::IfExpr(if_expr) => if_expr.span(),
            Self::AttrExpr(expr) => expr.span(),
            Self::EventTokens(closure) => closure.span(),
        }
    }
}

/// A if else chain attribute value
#[derive(PartialEq, Eq, Clone, Debug, Hash)]
pub struct IfAttributeValue {
    pub if_expr: ExprIf,
    pub condition: Expr,
    pub then_value: Box<AttributeValue>,
    pub else_value: Option<Box<AttributeValue>>,
}

impl IfAttributeValue {
    /// Convert the if expression to an expression that returns a string. If the unterminated case is hit, it returns an empty string
    pub(crate) fn quote_as_string(&self, diagnostics: &mut Diagnostics) -> Expr {
        let mut expression = quote! {};
        let mut current_if_value = self;

        let mut non_string_diagnostic = |span: proc_macro2::Span| -> Expr {
            Element::add_merging_non_string_diagnostic(diagnostics, span);
            parse_quote! { ::std::string::String::new() }
        };

        loop {
            let AttributeValue::AttrLiteral(lit) = current_if_value.then_value.as_ref() else {
                return non_string_diagnostic(current_if_value.span());
            };

            let HotLiteral::Fmted(HotReloadFormattedSegment {
                formatted_input: new,
                ..
            }) = &lit
            else {
                return non_string_diagnostic(current_if_value.span());
            };

            let condition = &current_if_value.if_expr.cond;
            expression.extend(quote! {
                if #condition {
                    #new.to_string()
                } else
            });
            match current_if_value.else_value.as_deref() {
                // If the else value is another if expression, then we need to continue the loop
                Some(AttributeValue::IfExpr(else_value)) => {
                    current_if_value = else_value;
                }
                // If the else value is a literal, then we need to append it to the expression and break
                Some(AttributeValue::AttrLiteral(lit)) => {
                    if let HotLiteral::Fmted(new) = &lit {
                        let fmted = &new.formatted_input;
                        expression.extend(quote! { { #fmted.to_string() } });
                        break;
                    } else {
                        return non_string_diagnostic(current_if_value.span());
                    }
                }
                // If it is the end of the if expression without an else, then we need to append the default value and break
                None => {
                    expression.extend(quote! { { ::std::string::String::new() } });
                    break;
                }
                _ => {
                    return non_string_diagnostic(current_if_value.else_value.span());
                }
            }
        }

        parse_quote! {
            {
                #expression
            }
        }
    }

    fn span(&self) -> Span {
        self.if_expr.span()
    }

    fn is_terminated(&self) -> bool {
        match &self.else_value {
            Some(attribute) => match attribute.as_ref() {
                AttributeValue::IfExpr(if_expr) => if_expr.is_terminated(),
                _ => true,
            },
            None => false,
        }
    }

    fn contains_expression(&self) -> bool {
        fn attribute_value_contains_expression(expr: &AttributeValue) -> bool {
            match expr {
                AttributeValue::IfExpr(if_expr) => if_expr.contains_expression(),
                AttributeValue::AttrLiteral(_) => false,
                _ => true,
            }
        }

        attribute_value_contains_expression(&self.then_value)
            || self
                .else_value
                .as_deref()
                .is_some_and(attribute_value_contains_expression)
    }

    fn parse_attribute_value_from_block(block: &Block) -> syn::Result<Box<AttributeValue>> {
        let stmts = &block.stmts;

        if stmts.len() != 1 {
            return Err(syn::Error::new(
                block.span(),
                "Expected a single statement in the if block",
            ));
        }

        // either an ifmt or an expr in the block
        let stmt = &stmts[0];

        // Either it's a valid ifmt or an expression
        match stmt {
            syn::Stmt::Expr(exp, None) => {
                // Try parsing the statement as an IfmtInput by passing it through tokens
                let value: Result<HotLiteral, syn::Error> = syn::parse2(exp.to_token_stream());
                Ok(match value {
                    Ok(res) => Box::new(AttributeValue::AttrLiteral(res)),
                    Err(_) => Box::new(AttributeValue::AttrExpr(PartialExpr::from_expr(exp))),
                })
            }
            _ => Err(syn::Error::new(stmt.span(), "Expected an expression")),
        }
    }

    fn to_tokens_with_terminated(
        &self,
        tokens: &mut TokenStream2,
        terminated: bool,
        contains_expression: bool,
    ) {
        let IfAttributeValue {
            if_expr,
            then_value,
            else_value,
            ..
        } = self;

        // Quote an attribute value and convert the value to a string if it is formatted
        // We always quote formatted segments as strings inside if statements so they have a consistent type
        // This fixes https://github.com/DioxusLabs/dioxus/issues/2997
        fn quote_attribute_value_string(
            value: &AttributeValue,
            contains_expression: bool,
        ) -> TokenStream2 {
            if let AttributeValue::AttrLiteral(HotLiteral::Fmted(fmted)) = value {
                if let Some(str) = fmted.to_static().filter(|_| contains_expression) {
                    // If this is actually a static string, the user may be using a static string expression in another branch
                    // use into to convert the string to whatever the other branch is using
                    quote! {
                        {
                            #[allow(clippy::useless_conversion)]
                            #str.into()
                        }
                    }
                } else {
                    quote! { #value.to_string() }
                }
            } else {
                value.to_token_stream()
            }
        }

        let then_value = quote_attribute_value_string(then_value, contains_expression);

        let then_value = if terminated {
            quote! { #then_value }
        }
        // Otherwise we need to return an Option and a None if the else value is None
        else {
            quote! { Some(#then_value) }
        };

        let else_value = match else_value.as_deref() {
            Some(AttributeValue::IfExpr(else_value)) => {
                let mut tokens = TokenStream2::new();
                else_value.to_tokens_with_terminated(&mut tokens, terminated, contains_expression);
                tokens
            }
            Some(other) => {
                let other = quote_attribute_value_string(other, contains_expression);
                if terminated {
                    other
                } else {
                    quote_spanned! { other.span() => Some(#other) }
                }
            }
            None => quote! { None },
        };

        let condition = &if_expr.cond;
        tokens.append_all(quote_spanned! { if_expr.span()=>
            if #condition {
                #then_value
            } else {
                #else_value
            }
        });
    }
}

impl Parse for IfAttributeValue {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let if_expr = input.parse::<ExprIf>()?;

        let stmts = &if_expr.then_branch.stmts;

        if stmts.len() != 1 {
            return Err(syn::Error::new(
                if_expr.then_branch.span(),
                "Expected a single statement in the if block",
            ));
        }

        // Parse the then branch into a single attribute value
        let then_value = Self::parse_attribute_value_from_block(&if_expr.then_branch)?;

        // If there's an else branch, parse it as a single attribute value or an if expression
        let else_value = match if_expr.else_branch.as_ref() {
            Some((_, else_branch)) => {
                // The else branch if either a block or another if expression
                let attribute_value = match else_branch.as_ref() {
                    // If it is a block, then the else is terminated
                    Expr::Block(block) => Self::parse_attribute_value_from_block(&block.block)?,
                    // Otherwise try to parse it as an if expression
                    _ => Box::new(syn::parse2(else_branch.to_token_stream())?),
                };
                Some(attribute_value)
            }
            None => None,
        };

        Ok(Self {
            condition: *if_expr.cond.clone(),
            if_expr,
            then_value,
            else_value,
        })
    }
}

impl ToTokens for IfAttributeValue {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        // If the if expression is terminated, we can just return the then value
        let terminated = self.is_terminated();
        let contains_expression = self.contains_expression();
        self.to_tokens_with_terminated(tokens, terminated, contains_expression)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;
    use syn::parse2;

    #[test]
    fn parse_attrs() {
        let _parsed: Attribute = parse2(quote! { name: "value" }).unwrap();
        let _parsed: Attribute = parse2(quote! { name: value }).unwrap();
        let _parsed: Attribute = parse2(quote! { name: "value {fmt}" }).unwrap();
        let _parsed: Attribute = parse2(quote! { name: 123 }).unwrap();
        let _parsed: Attribute = parse2(quote! { name: false }).unwrap();
        let _parsed: Attribute = parse2(quote! { "custom": false }).unwrap();
        let _parsed: Attribute = parse2(quote! { prop: "blah".to_string() }).unwrap();

        // with commas
        let _parsed: Attribute = parse2(quote! { "custom": false, }).unwrap();
        let _parsed: Attribute = parse2(quote! { name: false, }).unwrap();

        // with if chains
        let parsed: Attribute = parse2(quote! { name: if true { "value" } }).unwrap();
        assert!(matches!(parsed.value, AttributeValue::IfExpr(_)));
        let parsed: Attribute =
            parse2(quote! { name: if true { "value" } else { "other" } }).unwrap();
        assert!(matches!(parsed.value, AttributeValue::IfExpr(_)));
        let parsed: Attribute =
            parse2(quote! { name: if true { "value" } else if false { "other" } }).unwrap();
        assert!(matches!(parsed.value, AttributeValue::IfExpr(_)));

        // with shorthand
        let _parsed: Attribute = parse2(quote! { name }).unwrap();
        let _parsed: Attribute = parse2(quote! { name, }).unwrap();

        // Events - make sure they get partial expansion
        let parsed: Attribute = parse2(quote! { onclick: |e| {} }).unwrap();
        assert!(matches!(parsed.value, AttributeValue::EventTokens(_)));
        let parsed: Attribute = parse2(quote! { onclick: |e| { "value" } }).unwrap();
        assert!(matches!(parsed.value, AttributeValue::EventTokens(_)));
        let parsed: Attribute = parse2(quote! { onclick: |e| { value. } }).unwrap();
        assert!(matches!(parsed.value, AttributeValue::EventTokens(_)));
        let parsed: Attribute = parse2(quote! { onclick: move |e| { value. } }).unwrap();
        assert!(matches!(parsed.value, AttributeValue::EventTokens(_)));
        let parsed: Attribute = parse2(quote! { onclick: move |e| value }).unwrap();
        assert!(matches!(parsed.value, AttributeValue::EventTokens(_)));
        let parsed: Attribute = parse2(quote! { onclick: |e| value, }).unwrap();
        assert!(matches!(parsed.value, AttributeValue::EventTokens(_)));
    }

    #[test]
    fn merge_attrs() {
        let _a: Attribute = parse2(quote! { class: "value1" }).unwrap();
        let _b: Attribute = parse2(quote! { class: "value2" }).unwrap();

        let _b: Attribute = parse2(quote! { class: "value2 {something}" }).unwrap();
        let _b: Attribute = parse2(quote! { class: if value { "other thing" } }).unwrap();
        let _b: Attribute = parse2(quote! { class: if value { some_expr } }).unwrap();

        let _b: Attribute = parse2(quote! { class: if value { "some_expr" } }).unwrap();
        dbg!(_b);
    }

    #[test]
    fn static_literals() {
        let a: Attribute = parse2(quote! { class: "value1" }).unwrap();
        let b: Attribute = parse2(quote! { class: "value {some}" }).unwrap();

        assert!(a.is_static_str_literal());
        assert!(!b.is_static_str_literal());
    }

    #[test]
    fn partial_eqs() {
        // Basics
        let a: Attribute = parse2(quote! { class: "value1" }).unwrap();
        let b: Attribute = parse2(quote! { class: "value1" }).unwrap();
        assert_eq!(a, b);

        // Exprs
        let a: Attribute = parse2(quote! { class: var }).unwrap();
        let b: Attribute = parse2(quote! { class: var }).unwrap();
        assert_eq!(a, b);

        // Events
        let a: Attribute = parse2(quote! { onclick: |e| {} }).unwrap();
        let b: Attribute = parse2(quote! { onclick: |e| {} }).unwrap();
        let c: Attribute = parse2(quote! { onclick: move |e| {} }).unwrap();
        let d: Attribute = parse2(quote! { onclick: { |e| {} } }).unwrap();
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_ne!(a, d);
    }

    #[test]
    fn call_with_explicit_closure() {
        let mut a: Attribute = parse2(quote! { onclick: |e| {} }).unwrap();
        a.el_name = Some(parse_quote!(button));
        assert!(
            a.rendered_as_dynamic_attr()
                .to_string()
                .contains("call_with_explicit_closure")
        );

        let mut a: Attribute = parse2(quote! { onclick: { let a = 1; |e| {} } }).unwrap();
        a.el_name = Some(parse_quote!(button));
        assert!(
            a.rendered_as_dynamic_attr()
                .to_string()
                .contains("call_with_explicit_closure")
        );

        let mut a: Attribute = parse2(quote! { onclick: { let b = 2; { |e| { b } } } }).unwrap();
        a.el_name = Some(parse_quote!(button));
        assert!(
            a.rendered_as_dynamic_attr()
                .to_string()
                .contains("call_with_explicit_closure")
        );

        let mut a: Attribute = parse2(quote! { onclick: { let r = |e| { b }; r } }).unwrap();
        a.el_name = Some(parse_quote!(button));
        assert!(
            !a.rendered_as_dynamic_attr()
                .to_string()
                .contains("call_with_explicit_closure")
        );
    }

    /// Make sure reserved keywords are parsed as attributes
    /// HTML gets annoying sometimes so we just accept them
    #[test]
    fn reserved_keywords() {
        let _a: Attribute = parse2(quote! { for: "class" }).unwrap();
        let _b: Attribute = parse2(quote! { type: "class" }).unwrap();
    }
}
