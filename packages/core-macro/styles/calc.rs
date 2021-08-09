//! The `calc` functionality.
use crate::LengthPercentage;
use ::{
    proc_macro2::TokenStream,
    quote::{quote, ToTokens},
    std::fmt,
    syn::{
        custom_keyword, parenthesized,
        parse::{Parse, ParseStream},
        Token,
    },
};

/// Values that can be a calculaion (currently restricted to length & percentages)
#[derive(Debug, Clone, PartialEq)]
pub enum Calc {
    Calculated(CalcSum),
    Normal(LengthPercentage),
}

impl fmt::Display for Calc {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Calc::Calculated(inner) => write!(f, "calc({})", inner),
            Calc::Normal(inner) => write!(f, "{}", inner),
        }
    }
}

impl Parse for Calc {
    fn parse(s: ParseStream) -> syn::Result<Self> {
        custom_keyword!(calc);
        if s.peek(calc) {
            s.parse::<calc>()?;
            let content;
            parenthesized!(content in s);
            Ok(Calc::Calculated(content.parse()?))
        } else {
            Ok(Calc::Normal(s.parse()?))
        }
    }
}

impl ToTokens for Calc {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(match self {
            Calc::Calculated(inner) => quote!(style::Calc::Calculated(#inner)),
            Calc::Normal(inner) => quote!(style::Calc::Normal(#inner)),
        });
    }
}

#[test]
fn test_calc() {
    for (input, output) in vec![
        ("calc(10% - 20\"em\")", "calc(10% - 20em)"),
        ("calc(100% + 5px)", "calc(100% + 5px)"),
        ("calc(100% - 60px)", "calc(100% - 60px)"),
    ] {
        assert_eq!(&syn::parse_str::<Calc>(input).unwrap().to_string(), output);
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CalcSum {
    pub first: CalcProduct,
    pub rest: Vec<SumOp>,
}

impl fmt::Display for CalcSum {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.first)?;
        for op in self.rest.iter() {
            write!(f, "{}", op)?;
        }
        Ok(())
    }
}

impl Parse for CalcSum {
    fn parse(s: ParseStream) -> syn::Result<Self> {
        let first: CalcProduct = s.parse()?;
        let mut rest: Vec<SumOp> = vec![];
        while SumOp::peek(s) {
            rest.push(s.parse()?);
        }
        Ok(CalcSum { first, rest })
    }
}

impl ToTokens for CalcSum {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let first = &self.first;
        let rest = self.rest.iter();
        tokens.extend(quote! {
            style::calc::CalcSum {
                first: #first,
                rest: vec![#(#rest,)*]
            }
        });
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum SumOp {
    Add(CalcProduct),
    Sub(CalcProduct),
}

impl SumOp {
    fn peek(s: ParseStream) -> bool {
        s.peek(Token![+]) || s.peek(Token![-])
    }
}

impl fmt::Display for SumOp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SumOp::Add(inner) => write!(f, " + {}", inner),
            SumOp::Sub(inner) => write!(f, " - {}", inner),
        }
    }
}

impl Parse for SumOp {
    fn parse(s: ParseStream) -> syn::Result<Self> {
        let lookahead = s.lookahead1();
        if lookahead.peek(Token![+]) {
            s.parse::<Token![+]>()?;
            Ok(SumOp::Add(s.parse()?))
        } else if lookahead.peek(Token![-]) {
            s.parse::<Token![-]>()?;
            Ok(SumOp::Sub(s.parse()?))
        } else {
            Err(lookahead.error())
        }
    }
}

impl ToTokens for SumOp {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(match self {
            SumOp::Add(inner) => quote!(style::SumOp::Add(#inner)),
            SumOp::Sub(inner) => quote!(style::SumOp::Sub(#inner)),
        });
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CalcProduct {
    pub first: CalcValue,
    pub rest: Vec<ProductOp>,
}

impl fmt::Display for CalcProduct {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.first)?;
        for op in self.rest.iter() {
            write!(f, "{}", op)?;
        }
        Ok(())
    }
}

impl Parse for CalcProduct {
    fn parse(s: ParseStream) -> syn::Result<Self> {
        let first: CalcValue = s.parse()?;
        let mut rest: Vec<ProductOp> = vec![];
        while ProductOp::peek(s) {
            rest.push(s.parse()?);
        }
        Ok(CalcProduct { first, rest })
    }
}

impl ToTokens for CalcProduct {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let first = &self.first;
        let rest = self.rest.iter();
        tokens.extend(quote! {
            style::calc::CalcProduct {
                first: #first,
                rest: vec![#(#rest,)*]
            }
        });
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ProductOp {
    Mul(CalcValue),
    // todo Div(Number),
}

impl ProductOp {
    pub fn peek(s: ParseStream) -> bool {
        s.peek(Token![*]) // || s.peek(Token[/])
    }
}

impl fmt::Display for ProductOp {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ProductOp::Mul(inner) => write!(f, "*{}", inner),
            //ProductOp::Div(inner) => write!(f, "/{}", inner),
        }
    }
}

impl Parse for ProductOp {
    fn parse(s: ParseStream) -> syn::Result<Self> {
        let lookahead = s.lookahead1();
        if lookahead.peek(Token![*]) {
            s.parse::<Token![*]>()?;
            Ok(ProductOp::Mul(s.parse()?))
        /*
        } else if lookahead.peek(Token![/]) {
            s.parse::<Token![/]>()?;
            Ok(ProductOp::Div(s.parse()?))
        */
        } else {
            Err(lookahead.error())
        }
    }
}

impl ToTokens for ProductOp {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(match self {
            ProductOp::Mul(inner) => quote!(style::ProductOp::Mul(#inner)),
            //ProductOp::Div(inner) => quote!(style::ProductOp::Div(#inner)),
        });
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum CalcValue {
    LengthPercentage(LengthPercentage),
    // todo more variants
}

impl Parse for CalcValue {
    fn parse(s: ParseStream) -> syn::Result<Self> {
        Ok(CalcValue::LengthPercentage(s.parse()?))
    }
}

impl fmt::Display for CalcValue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CalcValue::LengthPercentage(inner) => write!(f, "{}", inner),
        }
    }
}

impl ToTokens for CalcValue {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.extend(match self {
            CalcValue::LengthPercentage(inner) => {
                quote!(style::CalcValue::LengthPercentage(#inner))
            }
        });
    }
}
