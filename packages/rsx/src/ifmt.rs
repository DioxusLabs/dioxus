use std::{collections::HashSet, str::FromStr};

use proc_macro2::{Span, TokenStream};

use quote::{quote, ToTokens, TokenStreamExt};
use syn::{
    parse::{Parse, ParseStream},
    *,
};

pub fn format_args_f_impl(input: IfmtInput) -> Result<TokenStream> {
    Ok(input.into_token_stream())
}

#[allow(dead_code)] // dumb compiler does not see the struct being used...
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct IfmtInput {
    pub source: Option<LitStr>,
    pub segments: Vec<Segment>,
}

impl IfmtInput {
    pub fn new_static(input: &str) -> Self {
        Self {
            source: None,
            segments: vec![Segment::Literal(input.to_string())],
        }
    }

    pub fn is_static(&self) -> bool {
        matches!(self.segments.as_slice(), &[Segment::Literal(_)] | &[])
    }
}

impl IfmtInput {
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
        })
    }
}

impl ToTokens for IfmtInput {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        // build format_literal
        let mut format_literal = String::new();
        let mut expr_counter = 0;
        for segment in self.segments.iter() {
            match segment {
                Segment::Literal(s) => format_literal += &s.replace('{', "{{").replace('}', "}}"),
                Segment::Formatted(FormattedSegment {
                    format_args,
                    segment,
                }) => {
                    format_literal += "{";
                    match segment {
                        FormattedSegmentType::Expr(_) => {
                            format_literal += &expr_counter.to_string();
                            expr_counter += 1;
                        }
                        FormattedSegmentType::Ident(ident) => {
                            format_literal += &ident.to_string();
                        }
                    }
                    format_literal += ":";
                    format_literal += format_args;
                    format_literal += "}";
                }
            }
        }

        let positional_args = self.segments.iter().filter_map(|seg| {
            if let Segment::Formatted(FormattedSegment {
                segment: FormattedSegmentType::Expr(expr),
                ..
            }) = seg
            {
                Some(expr)
            } else {
                None
            }
        });

        // remove duplicate idents
        let named_args_idents: HashSet<_> = self
            .segments
            .iter()
            .filter_map(|seg| {
                if let Segment::Formatted(FormattedSegment {
                    segment: FormattedSegmentType::Ident(ident),
                    ..
                }) = seg
                {
                    Some(ident)
                } else {
                    None
                }
            })
            .collect();
        let named_args = named_args_idents
            .iter()
            .map(|ident| quote!(#ident = #ident));

        quote! {
            format_args!(
                #format_literal
                #(, #positional_args)*
                #(, #named_args)*
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
