use proc_macro2::TokenStream;
use quote::ToTokens;
use std::hash::{Hash, Hasher};
use syn::{
    braced,
    parse::{Parse, ParseStream, Parser},
    punctuated::Punctuated,
    token, Attribute, Block, Expr, ExprBlock, Pat, PatType, Result, ReturnType, Token, Type,
};
use syn::{BoundLifetimes, ExprClosure};

/// A closure that has internals that might not be completely valid rust code but we want to interpret it regardless
#[derive(Debug, Clone)]
pub struct PartialClosure {
    pub attrs: Vec<Attribute>,
    pub lifetimes: Option<BoundLifetimes>,
    pub constness: Option<Token![const]>,
    pub movability: Option<Token![static]>,
    pub asyncness: Option<Token![async]>,
    pub capture: Option<Token![move]>,
    pub or1_token: Token![|],
    pub inputs: Punctuated<Pat, Token![,]>,
    pub or2_token: Token![|],
    pub output: ReturnType,
    pub brace_token: Option<syn::token::Brace>,
    pub body: TokenStream,
}

impl PartialEq for PartialClosure {
    fn eq(&self, other: &Self) -> bool {
        self.attrs == other.attrs
            && self.lifetimes == other.lifetimes
            && self.constness == other.constness
            && self.movability == other.movability
            && self.asyncness == other.asyncness
            && self.capture == other.capture
            && self.or1_token == other.or1_token
            && self.inputs == other.inputs
            && self.or2_token == other.or2_token
            && self.output == other.output
            && self.brace_token == other.brace_token
            && self.body.to_string() == other.body.to_string()
    }
}

impl Eq for PartialClosure {}
impl Hash for PartialClosure {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.attrs.hash(state);
        self.lifetimes.hash(state);
        self.constness.hash(state);
        self.movability.hash(state);
        self.asyncness.hash(state);
        self.capture.hash(state);
        self.or1_token.hash(state);
        self.inputs.hash(state);
        self.or2_token.hash(state);
        self.output.hash(state);
        self.brace_token.hash(state);
        self.body.to_string().hash(state);
    }
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

        let mut brace_token = None;
        let body = if input.peek(token::Brace) {
            let body;
            let brace = braced!(body in input);
            brace_token = Some(brace);
            body.parse()?
        } else {
            // todo: maybe parse incomplete until a delimiter (; or , or })
            let body: Expr = input.parse()?;
            body.to_token_stream()
        };

        Ok(PartialClosure {
            attrs: Vec::new(),
            lifetimes,
            constness,
            movability,
            asyncness,
            capture,
            or1_token,
            inputs,
            or2_token,
            output,
            brace_token,
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

        if let Some(brace_token) = &self.brace_token {
            brace_token.surround(tokens, |tokens| {
                self.body.to_tokens(tokens);
            });
        } else {
            self.body.to_tokens(tokens);
        }
    }
}

impl PartialClosure {
    pub fn as_expr(&self) -> Result<Expr> {
        self.as_expr_closure().map(|closure| Expr::Closure(closure))
    }

    /// Convert this partial closure into a full closure if it is valid
    /// Returns err if the internal tokens can't be parsed as a closure
    pub fn as_expr_closure(&self) -> Result<ExprClosure> {
        // If there's a brace token, we need to parse the body as a block
        // Otherwise we can parse it as an expression
        let body = match self.brace_token.as_ref() {
            Some(brace) => Expr::Block(ExprBlock {
                attrs: Vec::new(),
                label: None,
                block: Block {
                    brace_token: brace.clone(),
                    stmts: Block::parse_within.parse2(self.body.clone())?,
                },
            }),

            None => syn::parse2(self.body.clone())?,
        };

        Ok(ExprClosure {
            attrs: self.attrs.clone(),
            asyncness: self.asyncness.clone(),
            capture: self.capture.clone(),
            inputs: self.inputs.clone(),
            output: self.output.clone(),
            body: Box::new(body),
            lifetimes: self.lifetimes.clone(),
            constness: self.constness.clone(),
            movability: self.movability.clone(),
            or1_token: self.or1_token.clone(),
            or2_token: self.or2_token.clone(),
        })
    }
}

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
    use quote::{quote, TokenStreamExt};

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

    // hmmmm: todo: one day enable partial expansion on incomplete exprs
    // kinda hard
    #[test]
    fn parse_delim() {
        fn parse_non_delimited_group(input: ParseStream) -> Result<()> {
            let (toks, cursor) = input.cursor().token_tree().unwrap();
            println!("{:?}", toks);
            let (toks, cursor) = cursor.token_tree().unwrap();
            println!("{:?}", toks);
            let (toks, cursor) = cursor.token_tree().unwrap();
            println!("{:?}", toks);
            Ok(())
        }

        let toks = quote! {
            method.,
        };

        let o = syn::parse::Parser::parse2(parse_non_delimited_group, toks);

        let parses: Result<PartialClosure> = syn::parse2(quote! {
            |a, b| method.
        });

        // parse_non_delimited_group(syn::parse2(toks).unwrap()).unwrap();
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
