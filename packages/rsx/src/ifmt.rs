use std::str::FromStr;

use proc_macro2::{Span, TokenStream};

use quote::{quote, ToTokens};
use syn::{
    parse::{Parse, ParseStream},
    *,
};

pub fn format_args_f_impl(input: IfmtInput) -> Result<TokenStream> {
    // build format_literal
    let mut format_literal = String::new();
    let mut expr_counter = 0;
    for segment in input.segments.iter() {
        match segment {
            Segment::Literal(s) => format_literal += &s.replace('{', "{{").replace('}', "}}"),
            Segment::Formatted {
                format_args,
                segment,
            } => {
                format_literal += "{";
                match segment {
                    FormattedSegment::Expr(_) => {
                        format_literal += &expr_counter.to_string();
                        expr_counter += 1;
                    }
                    FormattedSegment::Ident(ident) => {
                        format_literal += &ident.to_string();
                    }
                }
                format_literal += ":";
                format_literal += format_args;
                format_literal += "}";
            }
        }
    }

    let positional_args = input.segments.iter().filter_map(|seg| {
        if let Segment::Formatted {
            segment: FormattedSegment::Expr(expr),
            ..
        } = seg
        {
            Some(expr)
        } else {
            None
        }
    });

    let named_args = input.segments.iter().filter_map(|seg| {
        if let Segment::Formatted {
            segment: FormattedSegment::Ident(ident),
            ..
        } = seg
        {
            Some(quote! {#ident = #ident})
        } else {
            None
        }
    });

    Ok(quote! {
        format_args!(
            #format_literal
            #(, #positional_args)*
            #(, #named_args)*
        )
    })
}

#[allow(dead_code)] // dumb compiler does not see the struct being used...
#[derive(Debug)]
pub struct IfmtInput {
    pub segments: Vec<Segment>,
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
                                segments.push(Segment::Formatted {
                                    format_args: current_format_args,
                                    segment: FormattedSegment::parse(&current_captured)?,
                                });
                                break;
                            }
                            current_format_args.push(c);
                        }
                        break;
                    }
                    if c == '}' {
                        segments.push(Segment::Formatted {
                            format_args: String::new(),
                            segment: FormattedSegment::parse(&current_captured)?,
                        });
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

#[derive(Debug)]
pub enum Segment {
    Literal(String),
    Formatted {
        format_args: String,
        segment: FormattedSegment,
    },
}

#[derive(Debug)]
pub enum FormattedSegment {
    Expr(Box<Expr>),
    Ident(Ident),
}

impl FormattedSegment {
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

impl ToTokens for FormattedSegment {
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
