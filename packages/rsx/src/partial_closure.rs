use crate::PartialExpr;
use proc_macro2::TokenStream;
use quote::ToTokens;
use std::hash::{Hash, Hasher};
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    Attribute, Expr, Pat, PatType, Result, ReturnType, Token, Type,
};
use syn::{BoundLifetimes, ExprClosure};

/// A closure whose body might not be valid rust code but we want to interpret it regardless.
/// This lets us provide expansions in way more cases than normal closures at the expense of an
/// increased mainteance burden and complexity.
///
/// We do our best to reuse the same logic from partial exprs for the body of the PartialClosure.
/// The code here is simply stolen from `syn::ExprClosure` and lightly modified to work with
/// PartialExprs. We only removed the attrs field and changed the body to be a PartialExpr.
/// Otherwise, it's a direct copy of the original.
#[derive(Debug, Clone)]
pub struct PartialClosure {
    pub lifetimes: Option<BoundLifetimes>,
    pub constness: Option<Token![const]>,
    pub movability: Option<Token![static]>,
    pub asyncness: Option<Token![async]>,
    pub capture: Option<Token![move]>,
    pub or1_token: Token![|],
    pub inputs: Punctuated<Pat, Token![,]>,
    pub or2_token: Token![|],
    pub output: ReturnType,
    pub body: PartialExpr,
}

impl Parse for PartialClosure {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let lifetimes: Option<BoundLifetimes> = input.parse()?;
        let constness: Option<Token![const]> = input.parse()?;
        let movability: Option<Token![static]> = input.parse()?;
        let asyncness: Option<Token![async]> = input.parse()?;
        let capture: Option<Token![move]> = input.parse()?;
        let or1_token: Token![|] = input.parse()?;

        let mut inputs = Punctuated::new();
        loop {
            if input.peek(Token![|]) {
                break;
            }
            let value = closure_arg(input)?;
            inputs.push_value(value);
            if input.peek(Token![|]) {
                break;
            }
            let punct: Token![,] = input.parse()?;
            inputs.push_punct(punct);
        }

        let or2_token: Token![|] = input.parse()?;

        let output = if input.peek(Token![->]) {
            let arrow_token: Token![->] = input.parse()?;
            let ty: Type = input.parse()?;
            ReturnType::Type(arrow_token, Box::new(ty))
        } else {
            ReturnType::Default
        };

        let body = PartialExpr::parse(input)?;

        Ok(PartialClosure {
            lifetimes,
            constness,
            movability,
            asyncness,
            capture,
            or1_token,
            inputs,
            or2_token,
            output,
            body,
        })
    }
}

impl ToTokens for PartialClosure {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.lifetimes.to_tokens(tokens);
        self.constness.to_tokens(tokens);
        self.movability.to_tokens(tokens);
        self.asyncness.to_tokens(tokens);
        self.capture.to_tokens(tokens);
        self.or1_token.to_tokens(tokens);
        self.inputs.to_tokens(tokens);
        self.or2_token.to_tokens(tokens);
        self.output.to_tokens(tokens);
        self.body.to_tokens(tokens);
    }
}

impl PartialEq for PartialClosure {
    fn eq(&self, other: &Self) -> bool {
        self.lifetimes == other.lifetimes
            && self.constness == other.constness
            && self.movability == other.movability
            && self.asyncness == other.asyncness
            && self.capture == other.capture
            && self.or1_token == other.or1_token
            && self.inputs == other.inputs
            && self.or2_token == other.or2_token
            && self.output == other.output
            && self.body == other.body
    }
}

impl Eq for PartialClosure {}
impl Hash for PartialClosure {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.lifetimes.hash(state);
        self.constness.hash(state);
        self.movability.hash(state);
        self.asyncness.hash(state);
        self.capture.hash(state);
        self.or1_token.hash(state);
        self.inputs.hash(state);
        self.or2_token.hash(state);
        self.output.hash(state);
        self.body.hash(state);
    }
}

impl PartialClosure {
    /// Convert this partial closure into a full closure if it is valid
    /// Returns err if the internal tokens can't be parsed as a closure
    pub fn as_expr(&self) -> Result<Expr> {
        let expr_closure = ExprClosure {
            attrs: Vec::new(),
            asyncness: self.asyncness,
            capture: self.capture,
            inputs: self.inputs.clone(),
            output: self.output.clone(),
            lifetimes: self.lifetimes.clone(),
            constness: self.constness,
            movability: self.movability,
            or1_token: self.or1_token,
            or2_token: self.or2_token,

            // try to lower the body to an expression - if might fail if it can't
            body: Box::new(self.body.as_expr()?),
        };

        Ok(Expr::Closure(expr_closure))
    }
}

/// This might look complex but it is just a ripoff of the `syn::ExprClosure` implementation. AFAIK
/// This code is not particularly accessible from outside syn... so it lives here. sorry
fn closure_arg(input: ParseStream) -> Result<Pat> {
    let attrs = input.call(Attribute::parse_outer)?;
    let mut pat = Pat::parse_single(input)?;

    if input.peek(Token![:]) {
        Ok(Pat::Type(PatType {
            attrs,
            pat: Box::new(pat),
            colon_token: input.parse()?,
            ty: input.parse()?,
        }))
    } else {
        match &mut pat {
            Pat::Const(pat) => pat.attrs = attrs,
            Pat::Ident(pat) => pat.attrs = attrs,
            Pat::Lit(pat) => pat.attrs = attrs,
            Pat::Macro(pat) => pat.attrs = attrs,
            Pat::Or(pat) => pat.attrs = attrs,
            Pat::Paren(pat) => pat.attrs = attrs,
            Pat::Path(pat) => pat.attrs = attrs,
            Pat::Range(pat) => pat.attrs = attrs,
            Pat::Reference(pat) => pat.attrs = attrs,
            Pat::Rest(pat) => pat.attrs = attrs,
            Pat::Slice(pat) => pat.attrs = attrs,
            Pat::Struct(pat) => pat.attrs = attrs,
            Pat::Tuple(pat) => pat.attrs = attrs,
            Pat::TupleStruct(pat) => pat.attrs = attrs,
            Pat::Wild(pat) => pat.attrs = attrs,
            Pat::Type(_) => unreachable!(),
            Pat::Verbatim(_) => {}
            _ => {}
        }
        Ok(pat)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;

    #[test]
    fn parses() {
        let doesnt_parse: Result<ExprClosure> = syn::parse2(quote! {
            |a, b| { method. }
        });

        // regular closures can't parse as partial closures
        assert!(doesnt_parse.is_err());

        let parses: Result<PartialClosure> = syn::parse2(quote! {
            |a, b| { method. }
        });

        // but ours can - we just can't format it out
        let parses = parses.unwrap();
        dbg!(parses.to_token_stream().to_string());
    }

    #[test]
    fn parses_real_world() {
        let parses: Result<PartialClosure> = syn::parse2(quote! {
            move |_| {
                let mut sidebar = SHOW_SIDEBAR.write();
                *sidebar = !*sidebar;
            }
        });

        // but ours can - we just can't format it out
        let parses = parses.unwrap();
        dbg!(parses.to_token_stream().to_string());
        parses.as_expr().unwrap();

        let parses: Result<PartialClosure> = syn::parse2(quote! {
            move |_| {
                rsx! {
                    div { class: "max-w-lg lg:max-w-2xl mx-auto mb-16 text-center",
                        "gomg"
                        "hi!!"
                        "womh"
                    }
                };
                println!("hi")
            }
        });
        parses.unwrap().as_expr().unwrap();
    }

    #[test]
    fn partial_eqs() {
        let a: PartialClosure = syn::parse2(quote! {
            move |e| {
                println!("clicked!");
            }
        })
        .unwrap();

        let b: PartialClosure = syn::parse2(quote! {
            move |e| {
                println!("clicked!");
            }
        })
        .unwrap();

        let c: PartialClosure = syn::parse2(quote! {
            move |e| {
                println!("unclicked");
            }
        })
        .unwrap();

        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    /// Ensure our ToTokens impl is the same as the one in syn
    #[test]
    fn same_to_tokens() {
        let a: PartialClosure = syn::parse2(quote! {
            move |e| {
                println!("clicked!");
            }
        })
        .unwrap();

        let b: PartialClosure = syn::parse2(quote! {
            move |e| {
                println!("clicked!");
            }
        })
        .unwrap();

        let c: ExprClosure = syn::parse2(quote! {
            move |e| {
                println!("clicked!");
            }
        })
        .unwrap();

        assert_eq!(
            a.to_token_stream().to_string(),
            b.to_token_stream().to_string()
        );

        assert_eq!(
            a.to_token_stream().to_string(),
            c.to_token_stream().to_string()
        );

        let a: PartialClosure = syn::parse2(quote! {
            move |e| println!("clicked!")
        })
        .unwrap();

        let b: ExprClosure = syn::parse2(quote! {
            move |e| println!("clicked!")
        })
        .unwrap();

        assert_eq!(
            a.to_token_stream().to_string(),
            b.to_token_stream().to_string()
        );
    }
}
