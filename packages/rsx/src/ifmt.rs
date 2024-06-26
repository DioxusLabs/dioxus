use crate::intern;
use dioxus_core::prelude::{FmtSegment, FmtedSegments};
use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned, ToTokens, TokenStreamExt};
use std::{collections::HashMap, str::FromStr};
use syn::{
    parse::{Parse, ParseStream},
    *,
};

/// A hot-reloadable formatted string, boolean, number or other literal
#[derive(Debug, Eq, Clone, Hash)]
pub struct IfmtInput {
    pub source: LitStr,
    pub segments: Vec<Segment>,
}

// Specifically avoid colliding the location field in partialeq
// This is just because we usually want to compare two ifmts with different locations just based on their contents
impl PartialEq for IfmtInput {
    fn eq(&self, other: &Self) -> bool {
        self.source == other.source && self.segments == other.segments
    }
}

impl IfmtInput {
    pub fn new_litstr(source: LitStr) -> Self {
        let segments = Self::from_raw(&source.value()).unwrap();
        Self { segments, source }
    }

    pub fn span(&self) -> Span {
        self.source.span()
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

    fn dynamic_segments(&self) -> Vec<&FormattedSegment> {
        self.segments
            .iter()
            .filter_map(|seg| match seg {
                Segment::Formatted(seg) => Some(seg),
                _ => None,
            })
            .collect::<Vec<_>>()
    }

    fn dynamic_seg_frequency_map(&self) -> HashMap<&FormattedSegment, usize> {
        let mut map = HashMap::new();
        for seg in self.dynamic_segments() {
            *map.entry(seg).or_insert(0) += 1;
        }
        map
    }

    pub fn hr_score(&self, other: &Self) -> usize {
        // If they're the same by source, return max
        if self == other {
            return usize::MAX;
        }

        // Default score to 1 - an ifmt with no dynamic segments still technically has a score of 1
        // since it's not disqualified, but it's not a perfect match
        let mut score = 1;
        let mut l_freq_map = self.dynamic_seg_frequency_map();

        // Pluck out the dynamic segments from the other input
        for seg in other.dynamic_segments() {
            let Some(ct) = l_freq_map.get_mut(seg) else {
                return 0;
            };

            *ct -= 1;

            if *ct == 0 {
                l_freq_map.remove(seg);
            }

            score += 1;
        }

        // If there's nothing remaining - a perfect match - return max -1
        // We compared the sources to start, so we know they're different in some way
        if l_freq_map.is_empty() {
            usize::MAX - 1
        } else {
            score
        }
    }

    pub fn fmt_segments(&self, other: &Self) -> Option<FmtedSegments> {
        let a = self;
        let b = other;

        // Make sure all the dynamic segments of b show up in a
        for segment in b.segments.iter() {
            if segment.is_formatted() && !a.segments.contains(segment) {
                return None;
            }
        }

        // Collect all the formatted segments from the original
        let mut out = vec![];

        // the original list of formatted segments
        let mut fmted = a
            .segments
            .iter()
            .flat_map(|f| match f {
                crate::Segment::Literal(_) => None,
                crate::Segment::Formatted(f) => Some(f),
            })
            .cloned()
            .map(|f| Some(f))
            .collect::<Vec<_>>();

        for segment in b.segments.iter() {
            match segment {
                crate::Segment::Literal(lit) => {
                    // create a &'static str by leaking the string
                    let lit = intern(lit.clone().into_boxed_str());
                    out.push(FmtSegment::Literal { value: lit });
                }
                crate::Segment::Formatted(fmt) => {
                    // Find the formatted segment in the original
                    // Set it to None when we find it so we don't re-render it on accident
                    let idx = fmted
                        .iter_mut()
                        .position(|_s| {
                            if let Some(s) = _s {
                                if s == fmt {
                                    *_s = None;
                                    return true;
                                }
                            }

                            false
                        })
                        .unwrap();

                    out.push(FmtSegment::Dynamic { id: idx });
                }
            }
        }

        Some(FmtedSegments::new(out))
    }

    fn is_simple_expr(&self) -> bool {
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
    pub fn to_quoted_string_from_parts(&self) -> String {
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
        // Try to turn it into a single _.to_string() call
        if !cfg!(debug_assertions) {
            if let Some(single_dynamic) = self.try_to_string() {
                tokens.extend(single_dynamic);
                return;
            }
        }

        // If the segments are not complex exprs, we can just use format! directly to take advantage of RA rename/expansion
        if self.is_simple_expr() {
            let raw = &self.source;
            tokens.extend(quote! {
                ::std::format_args!(#raw)
            });
            return;
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
            ::std::format_args!(
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
    format_args: String,
    segment: FormattedSegmentType,
}

impl ToTokens for FormattedSegment {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let (fmt, seg) = (&self.format_args, &self.segment);
        let fmt = format!("{{0:{fmt}}}");
        tokens.append_all(quote! {
            format_args!(#fmt, #seg)
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
        if let Ok(ident) = parse_str::<Ident>(input) {
            if ident == input {
                return Ok(Self::Ident(ident));
            }
        }
        if let Ok(expr) = parse_str(input) {
            Ok(Self::Expr(Box::new(expr)))
        } else {
            Err(Error::new(
                Span::call_site(),
                "Expected Ident or Expression",
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

impl FromStr for IfmtInput {
    type Err = syn::Error;

    fn from_str(input: &str) -> Result<Self> {
        let segments = IfmtInput::from_raw(input)?;
        Ok(Self {
            source: LitStr::new(input, Span::call_site()),
            segments,
        })
    }
}

impl Parse for IfmtInput {
    fn parse(input: ParseStream) -> Result<Self> {
        let source: LitStr = input.parse()?;
        let segments = IfmtInput::from_raw(&source.value())?;
        Ok(Self { source, segments })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reload_stack::ReloadStack;
    use crate::PrettyUnparse;

    /// Ensure the scoring algorithm works
    ///
    /// - usize::MAX is return for perfect overlap
    /// - 0 is returned when the right case has segments not found in the first
    /// - a number for the other cases where there is some non-perfect overlap
    #[test]
    fn ifmt_scoring() {
        let left: IfmtInput = "{abc} {def}".parse().unwrap();
        let right: IfmtInput = "{abc}".parse().unwrap();
        assert_eq!(left.hr_score(&right), 2);

        let left: IfmtInput = "{abc} {def}".parse().unwrap();
        let right: IfmtInput = "{abc} {def}".parse().unwrap();
        assert_eq!(left.hr_score(&right), usize::MAX);

        let left: IfmtInput = "{abc} {def}".parse().unwrap();
        let right: IfmtInput = "{abc} {ghi}".parse().unwrap();
        assert_eq!(left.hr_score(&right), 0);

        let left: IfmtInput = "{abc} {def}".parse().unwrap();
        let right: IfmtInput = "{abc} {def} {ghi}".parse().unwrap();
        assert_eq!(left.hr_score(&right), 0);

        let left: IfmtInput = "{abc} {def} {ghi}".parse().unwrap();
        let right: IfmtInput = "{abc} {def}".parse().unwrap();
        assert_eq!(left.hr_score(&right), 3);

        let left: IfmtInput = "{abc}".parse().unwrap();
        let right: IfmtInput = "{abc} {def}".parse().unwrap();
        assert_eq!(left.hr_score(&right), 0);

        let left: IfmtInput = "{abc} {abc} {def}".parse().unwrap();
        let right: IfmtInput = "{abc} {def}".parse().unwrap();
        assert_eq!(left.hr_score(&right), 3);

        let left: IfmtInput = "{abc} {abc}".parse().unwrap();
        let right: IfmtInput = "{abc} {abc}".parse().unwrap();
        assert_eq!(left.hr_score(&right), usize::MAX);

        let left: IfmtInput = "{abc} {def}".parse().unwrap();
        let right: IfmtInput = "{hij}".parse().unwrap();
        assert_eq!(left.hr_score(&right), 0);

        let left: IfmtInput = "{abc}".parse().unwrap();
        let right: IfmtInput = "thing {abc}".parse().unwrap();
        assert_eq!(left.hr_score(&right), usize::MAX - 1);

        let left: IfmtInput = "thing {abc}".parse().unwrap();
        let right: IfmtInput = "{abc}".parse().unwrap();
        assert_eq!(left.hr_score(&right), usize::MAX - 1);

        let left: IfmtInput = "{abc} {def}".parse().unwrap();
        let right: IfmtInput = "thing {abc}".parse().unwrap();
        assert_eq!(left.hr_score(&right), 2);
    }

    #[test]
    fn stack_scoring() {
        let mut stack: ReloadStack<IfmtInput> = ReloadStack::new(
            vec![
                "{abc} {def}".parse().unwrap(),
                "{def}".parse().unwrap(),
                "{hij}".parse().unwrap(),
            ]
            .into_iter(),
        );

        let tests = vec![
            //
            "thing {def}",
            "thing {abc}",
            "thing {hij}",
        ];

        for item in tests {
            let score = stack.highest_score(|f| f.hr_score(&item.parse().unwrap()));

            dbg!(item, score);
        }
    }

    #[test]
    fn raw_tokens() {
        let input = syn::parse2::<IfmtInput>(quote! { r#"hello world"# }).unwrap();
        println!("{}", input.to_token_stream().pretty_unparse());
        assert_eq!(input.source.value(), "hello world");
    }

    #[test]
    fn segments_parse() {
        let input = "blah {abc} {def}".parse::<IfmtInput>().unwrap();
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
        println!("{}", input.to_quoted_string_from_parts());

        let input = syn::parse2::<IfmtInput>(quote! { "hello {world} {world} {world}" }).unwrap();
        println!("{}", input.to_quoted_string_from_parts());

        let input = syn::parse2::<IfmtInput>(quote! { "hello {world} {world} {world()}" }).unwrap();
        println!("{}", input.to_quoted_string_from_parts());

        let input =
            syn::parse2::<IfmtInput>(quote! { r#"hello {world} {world} {world()}"# }).unwrap();
        println!("{}", input.to_quoted_string_from_parts());

        let input = syn::parse2::<IfmtInput>(quote! { r#"hello"# }).unwrap();
        println!("{}", input.to_quoted_string_from_parts());
    }
}
