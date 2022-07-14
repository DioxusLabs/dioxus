use std::str::FromStr;

use proc_macro2::{Span, TokenStream};

use quote::{quote, ToTokens};
use syn::{
    parse::{Parse, ParseStream},
    *,
};

pub fn format_args_f_impl(input: IfmtInput) -> Result<TokenStream> {
    Ok(input.into_token_stream())
}

#[allow(dead_code)] // dumb compiler does not see the struct being used...
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct IfmtInput {
    pub segments: Vec<Segment>,
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
                segments.push(Segment::Literal(current_literal));
                current_literal = String::new();
                let mut current_captured = String::new();
                while let Some(c) = chars.next() {
                    if c == ':' {
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
        segments.push(Segment::Literal(current_literal));
        Ok(Self { segments })
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

        let named_args = self.segments.iter().filter_map(|seg| {
            if let Segment::Formatted(FormattedSegment {
                segment: FormattedSegmentType::Ident(ident),
                ..
            }) = seg
            {
                Some(quote! {#ident = #ident})
            } else {
                None
            }
        });

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

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Segment {
    Literal(String),
    Formatted(FormattedSegment),
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct FormattedSegment {
    format_args: String,
    segment: FormattedSegmentType,
}

#[derive(Debug, PartialEq, Eq, Clone)]
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
        IfmtInput::from_str(&input_str)
    }
}
