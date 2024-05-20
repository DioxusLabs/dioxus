use crate::location::CallerLocation;
use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned, ToTokens, TokenStreamExt};
use std::str::FromStr;
use syn::{
    parse::{Parse, ParseStream},
    *,
};

#[derive(Debug, Eq, Clone, Hash, Default)]
pub struct IfmtInput {
    pub source: Option<LitStr>,
    pub segments: Vec<Segment>,
    pub hr_idx: CallerLocation,
}

// Specifically avoid colliding the location field in partialeq
// This is just because we usually want to compare two ifmts with different locations just based on their contents
impl PartialEq for IfmtInput {
    fn eq(&self, other: &Self) -> bool {
        self.source == other.source && self.segments == other.segments
    }
}

impl IfmtInput {
    pub fn new_static(input: &str) -> Self {
        Self {
            source: None,
            segments: vec![Segment::Literal(input.to_string())],
            hr_idx: Default::default(),
        }
    }

    pub fn join(mut self, other: Self, separator: &str) -> Self {
        if !self.segments.is_empty() {
            self.segments.push(Segment::Literal(separator.to_string()));
        }
        self.segments.extend(other.segments);
        self
    }

    pub fn push_expr(&mut self, expr: Expr) {
        self.segments.push(Segment::Formatted(FormattedSegment {
            format_args: String::new(),
            segment: FormattedSegmentType::Expr(Box::new(expr)),
        }));
    }

    pub fn push_str(&mut self, s: &str) {
        self.segments.push(Segment::Literal(s.to_string()));
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

    pub fn as_htotreloaded(&self) -> TokenStream {
        let mut idx = 0_usize;
        let segments = self.segments.iter().map(|s| match s {
            Segment::Literal(lit) => quote! {
                FmtSegment::Literal { value: #lit }
            },
            Segment::Formatted(_fmt) => {
                // increment idx for the dynamic segment so we maintain the mapping
                let _idx = idx;
                idx += 1;
                quote! {
                    FmtSegment::Dynamic { id: #_idx }
                }
            }
        });

        quote! {
            FmtedSegments::new(
                // The static segments with idxs for locations
                vec![ #(#segments),* ],
            )
        }
        .to_token_stream()
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
}

impl FromStr for IfmtInput {
    type Err = syn::Error;

    fn from_str(input: &str) -> Result<Self> {
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

        Ok(Self {
            segments,
            source: None,
            hr_idx: Default::default(),
        })
    }
}

impl ToTokens for IfmtInput {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        // Try to turn it into a single _.to_string() call
        if let Some(single_dynamic) = self.try_to_string() {
            tokens.extend(single_dynamic);
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

        let span = match self.source.as_ref() {
            Some(source) => source.span(),
            None => Span::call_site(),
        };

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

impl Parse for IfmtInput {
    fn parse(input: ParseStream) -> Result<Self> {
        let input: LitStr = input.parse()?;
        let input_str = input.value();
        let mut ifmt = IfmtInput::from_str(&input_str)?;
        ifmt.source = Some(input);
        Ok(ifmt)
    }
}

fn hmm() {
    // // place down the signal stuff

    // let segments = txt.as_htotreloaded();

    // let rendered_segments = txt.segments.iter().filter_map(|s| match s {
    //     Segment::Literal(lit) => None,
    //     Segment::Formatted(fmt) => {
    //         // just render as a format_args! call
    //         Some(quote! {
    //             format!("{}", #fmt)
    //         })
    //     }
    // });

    // let old_idx = self.location.idx.get();
    // let cur_idx = (old_idx) * 100000 + 1 + idx;

    // quote! {
    //     {
    //         // Create a signal of the formatted segments
    //         // htotreloading will find this via its location and then update the signal
    //         static __SIGNAL: GlobalSignal<FmtedSegments> = GlobalSignal::with_key(|| #segments, {
    //             concat!(
    //                 file!(),
    //                 ":",
    //                 line!(),
    //                 ":",
    //                 column!(),
    //                 ":",
    //                 #cur_idx
    //             )
    //         });

    //         // render the signal and subscribe the component to its changes
    //         __SIGNAL.with(|s| s.render_with(
    //             vec![ #(#rendered_segments),* ]
    //         ))
    //     }
    // }
}
