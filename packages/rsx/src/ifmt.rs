use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned, ToTokens, TokenStreamExt};
use std::collections::HashMap;
use syn::{
    parse::{Parse, ParseStream},
    *,
};

/// A hot-reloadable formatted string, boolean, number or other literal
///
/// This wraps LitStr with some extra goodies like inline expressions and hot-reloading.
/// Originally this was intended to provide named inline string interpolation but eventually Rust
/// actually shipped this!
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct IfmtInput {
    pub source: LitStr,
    pub segments: Vec<Segment>,
}

impl IfmtInput {
    pub fn new(span: Span) -> Self {
        Self {
            source: LitStr::new("", span),
            segments: Vec::new(),
        }
    }

    pub fn new_litstr(source: LitStr) -> Result<Self> {
        let segments = IfmtInput::from_raw(&source.value()).map_err(|e| {
            // If there is an error creating the formatted string, attribute it to the litstr span
            let span = source.span();
            syn::Error::new(span, e)
        })?;
        Ok(Self { segments, source })
    }

    pub fn span(&self) -> Span {
        self.source.span()
    }

    pub fn push_raw_str(&mut self, other: String) {
        self.segments.push(Segment::Literal(other.to_string()))
    }

    pub fn push_ifmt(&mut self, other: IfmtInput) {
        self.segments.extend(other.segments);
    }

    pub fn push_expr(&mut self, expr: Expr) {
        self.segments.push(Segment::Formatted(FormattedSegment {
            format_args: String::new(),
            segment: FormattedSegmentType::Expr(Box::new(expr)),
        }));
    }

    pub fn is_static(&self) -> bool {
        self.segments
            .iter()
            .all(|seg| matches!(seg, Segment::Literal(_)))
    }

    pub fn to_static(&self) -> Option<String> {
        self.segments
            .iter()
            .try_fold(String::new(), |acc, segment| {
                if let Segment::Literal(seg) = segment {
                    Some(acc + seg)
                } else {
                    None
                }
            })
    }

    pub fn dynamic_segments(&self) -> Vec<&FormattedSegment> {
        self.segments
            .iter()
            .filter_map(|seg| match seg {
                Segment::Formatted(seg) => Some(seg),
                _ => None,
            })
            .collect::<Vec<_>>()
    }

    pub fn dynamic_seg_frequency_map(&self) -> HashMap<&FormattedSegment, usize> {
        let mut map = HashMap::new();
        for seg in self.dynamic_segments() {
            *map.entry(seg).or_insert(0) += 1;
        }
        map
    }

    fn is_simple_expr(&self) -> bool {
        // If there are segments but the source is empty, it's not a simple expression.
        if !self.segments.is_empty() && self.source.span().byte_range().is_empty() {
            return false;
        }

        self.segments.iter().all(|seg| match seg {
            Segment::Literal(_) => true,
            Segment::Formatted(FormattedSegment { segment, .. }) => {
                matches!(segment, FormattedSegmentType::Ident(_))
            }
        })
    }

    /// Try to convert this into a single _.to_string() call if possible
    ///
    /// Using "{single_expression}" is pretty common, but you don't need to go through the whole format! machinery for that, so we optimize it here.
    fn try_to_string(&self) -> Option<TokenStream> {
        let mut single_dynamic = None;
        for segment in &self.segments {
            match segment {
                Segment::Literal(literal) => {
                    if !literal.is_empty() {
                        return None;
                    }
                }
                Segment::Formatted(FormattedSegment {
                    segment,
                    format_args,
                }) => {
                    if format_args.is_empty() {
                        match single_dynamic {
                            Some(current_string) => {
                                single_dynamic =
                                    Some(quote!(#current_string + &(#segment).to_string()));
                            }
                            None => {
                                single_dynamic = Some(quote!((#segment).to_string()));
                            }
                        }
                    } else {
                        return None;
                    }
                }
            }
        }
        single_dynamic
    }

    /// print the original source string - this handles escapes and stuff for us
    pub fn to_string_with_quotes(&self) -> String {
        self.source.to_token_stream().to_string()
    }

    /// Parse the source into segments
    fn from_raw(input: &str) -> Result<Vec<Segment>> {
        let mut chars = input.chars().peekable();
        let mut segments = Vec::new();
        let mut current_literal = String::new();
        while let Some(c) = chars.next() {
            if c == '{' {
                if let Some(c) = chars.next_if(|c| *c == '{') {
                    current_literal.push(c);
                    continue;
                }
                if !current_literal.is_empty() {
                    segments.push(Segment::Literal(current_literal));
                }
                current_literal = String::new();
                let mut current_captured = String::new();
                while let Some(c) = chars.next() {
                    if c == ':' {
                        // two :s in a row is a path, not a format arg
                        if chars.next_if(|c| *c == ':').is_some() {
                            current_captured.push_str("::");
                            continue;
                        }
                        let mut current_format_args = String::new();
                        for c in chars.by_ref() {
                            if c == '}' {
                                segments.push(Segment::Formatted(FormattedSegment {
                                    format_args: current_format_args,
                                    segment: FormattedSegmentType::parse(&current_captured)?,
                                }));
                                break;
                            }
                            current_format_args.push(c);
                        }
                        break;
                    }
                    if c == '}' {
                        segments.push(Segment::Formatted(FormattedSegment {
                            format_args: String::new(),
                            segment: FormattedSegmentType::parse(&current_captured)?,
                        }));
                        break;
                    }
                    current_captured.push(c);
                }
            } else {
                if '}' == c {
                    if let Some(c) = chars.next_if(|c| *c == '}') {
                        current_literal.push(c);
                        continue;
                    } else {
                        return Err(Error::new(
                            Span::call_site(),
                            "unmatched closing '}' in format string",
                        ));
                    }
                }
                current_literal.push(c);
            }
        }

        if !current_literal.is_empty() {
            segments.push(Segment::Literal(current_literal));
        }

        Ok(segments)
    }
}

impl ToTokens for IfmtInput {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        // If the input is a string literal, we can just return it
        if let Some(static_str) = self.to_static() {
            return quote_spanned! { self.span() => #static_str }.to_tokens(tokens);
        }

        // Try to turn it into a single _.to_string() call
        if !cfg!(debug_assertions)
            && let Some(single_dynamic) = self.try_to_string()
        {
            tokens.extend(single_dynamic);
            return;
        }

        // If the segments are not complex exprs, we can just use format! directly to take advantage of RA rename/expansion
        if self.is_simple_expr() {
            let raw = &self.source;
            return quote_spanned! { raw.span() => ::std::format!(#raw) }.to_tokens(tokens);
        }

        // build format_literal
        let mut format_literal = String::new();
        let mut expr_counter = 0;
        for segment in self.segments.iter() {
            match segment {
                Segment::Literal(s) => format_literal += &s.replace('{', "{{").replace('}', "}}"),
                Segment::Formatted(FormattedSegment { format_args, .. }) => {
                    format_literal += "{";
                    format_literal += &expr_counter.to_string();
                    expr_counter += 1;
                    format_literal += ":";
                    format_literal += format_args;
                    format_literal += "}";
                }
            }
        }

        let span = self.span();

        let positional_args = self.segments.iter().filter_map(|seg| {
            if let Segment::Formatted(FormattedSegment { segment, .. }) = seg {
                let mut segment = segment.clone();
                // We set the span of the ident here, so that we can use it in diagnostics
                if let FormattedSegmentType::Ident(ident) = &mut segment {
                    ident.set_span(span);
                }
                Some(segment)
            } else {
                None
            }
        });

        quote_spanned! {
            span =>
            ::std::format!(
                #format_literal
                #(, #positional_args)*
            )
        }
        .to_tokens(tokens)
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub enum Segment {
    Literal(String),
    Formatted(FormattedSegment),
}

impl Segment {
    pub fn is_literal(&self) -> bool {
        matches!(self, Segment::Literal(_))
    }

    pub fn is_formatted(&self) -> bool {
        matches!(self, Segment::Formatted(_))
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct FormattedSegment {
    pub format_args: String,
    pub segment: FormattedSegmentType,
}

impl ToTokens for FormattedSegment {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let (fmt, seg) = (&self.format_args, &self.segment);
        let fmt = format!("{{0:{fmt}}}");
        tokens.append_all(quote! {
            format!(#fmt, #seg)
        });
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub enum FormattedSegmentType {
    Expr(Box<Expr>),
    Ident(Ident),
}

impl FormattedSegmentType {
    fn parse(input: &str) -> Result<Self> {
        if let Ok(ident) = parse_str::<Ident>(input)
            && ident == input
        {
            return Ok(Self::Ident(ident));
        }
        if let Ok(expr) = parse_str(input) {
            Ok(Self::Expr(Box::new(expr)))
        } else {
            Err(Error::new(
                Span::call_site(),
                "Failed to parse formatted segment: Expected Ident or Expression",
            ))
        }
    }
}

impl ToTokens for FormattedSegmentType {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        match self {
            Self::Expr(expr) => expr.to_tokens(tokens),
            Self::Ident(ident) => ident.to_tokens(tokens),
        }
    }
}

impl Parse for IfmtInput {
    fn parse(input: ParseStream) -> Result<Self> {
        let source: LitStr = input.parse()?;
        Self::new_litstr(source)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use prettier_please::PrettyUnparse;

    #[test]
    fn raw_tokens() {
        let input = syn::parse2::<IfmtInput>(quote! { r#"hello world"# }).unwrap();
        println!("{}", input.to_token_stream().pretty_unparse());
        assert_eq!(input.source.value(), "hello world");
        assert_eq!(input.to_string_with_quotes(), "r#\"hello world\"#");
    }

    #[test]
    fn segments_parse() {
        let input: IfmtInput = parse_quote! { "blah {abc} {def}" };
        assert_eq!(
            input.segments,
            vec![
                Segment::Literal("blah ".to_string()),
                Segment::Formatted(FormattedSegment {
                    format_args: String::new(),
                    segment: FormattedSegmentType::Ident(Ident::new("abc", Span::call_site()))
                }),
                Segment::Literal(" ".to_string()),
                Segment::Formatted(FormattedSegment {
                    format_args: String::new(),
                    segment: FormattedSegmentType::Ident(Ident::new("def", Span::call_site()))
                }),
            ]
        );
    }

    #[test]
    fn printing_raw() {
        let input = syn::parse2::<IfmtInput>(quote! { "hello {world}" }).unwrap();
        println!("{}", input.to_string_with_quotes());

        let input = syn::parse2::<IfmtInput>(quote! { "hello {world} {world} {world}" }).unwrap();
        println!("{}", input.to_string_with_quotes());

        let input = syn::parse2::<IfmtInput>(quote! { "hello {world} {world} {world()}" }).unwrap();
        println!("{}", input.to_string_with_quotes());

        let input =
            syn::parse2::<IfmtInput>(quote! { r#"hello {world} {world} {world()}"# }).unwrap();
        println!("{}", input.to_string_with_quotes());
        assert!(!input.is_static());

        let input = syn::parse2::<IfmtInput>(quote! { r#"hello"# }).unwrap();
        println!("{}", input.to_string_with_quotes());
        assert!(input.is_static());
    }

    #[test]
    fn to_static() {
        let input = syn::parse2::<IfmtInput>(quote! { "body {{ background: red; }}" }).unwrap();
        assert_eq!(
            input.to_static(),
            Some("body { background: red; }".to_string())
        );
    }

    #[test]
    fn error_spans() {
        let input = syn::parse2::<IfmtInput>(quote! { "body {{ background: red; }" }).unwrap_err();
        assert_eq!(input.span().byte_range(), 0..28);
    }
}
