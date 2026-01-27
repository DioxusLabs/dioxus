use proc_macro2::{Delimiter, TokenStream as TokenStream2, TokenTree};
use quote::ToTokens;
use std::hash;
use syn::{Expr, parse::Parse, spanned::Spanned, token::Brace};

/// A raw expression potentially wrapped in curly braces that is parsed from the input stream.
///
/// If there are no braces, it tries to parse as an expression without partial expansion. If there
/// are braces, it parses the contents as a `TokenStream2` and stores it as such.
#[derive(Clone, Debug)]
pub struct PartialExpr {
    pub brace: Option<Brace>,
    pub expr: TokenStream2,
}

impl Parse for PartialExpr {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        // Input is a braced expression if it's a braced group
        // followed by either of the following:
        // - the end of the stream
        // - a comma
        // - another braced group
        // - an identifier
        // - a string literal
        let mut is_braced = false;
        if let Some((TokenTree::Group(group), next)) = input.fork().cursor().token_tree() {
            let next_char_is_a_comma = next.punct().is_some_and(|(tt, _)| tt.as_char() == ',');
            let next_is_a_braced_exp = next.group(Delimiter::Brace).is_some();
            let next_is_an_ident = next.ident().is_some();
            let next_is_a_string_literal = next.literal().is_some();

            if group.delimiter() == Delimiter::Brace
                && (next.eof()
                    || next_char_is_a_comma
                    || next_is_a_braced_exp
                    || next_is_an_ident
                    || next_is_a_string_literal)
            {
                is_braced = true
            }
        };

        // Parse as an expression if it's not braced
        if !is_braced {
            let expr = input.parse::<syn::Expr>()?;
            return Ok(Self {
                brace: None,
                expr: expr.to_token_stream(),
            });
        }

        let content;
        let brace = Some(syn::braced!(content in input));
        let expr: TokenStream2 = content.parse()?;

        Ok(Self { brace, expr })
    }
}

impl ToTokens for PartialExpr {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        match &self.brace {
            Some(brace) => brace.surround(tokens, |tokens| self.expr.to_tokens(tokens)),
            _ => self.expr.to_tokens(tokens),
        }
    }
}

impl PartialExpr {
    pub fn as_expr(&self) -> syn::Result<syn::Expr> {
        // very important: make sure to include the brace in the span of the expr
        // otherwise autofmt will freak out since it will use the inner span
        if let Some(brace) = &self.brace {
            let mut tokens = TokenStream2::new();
            let f = |tokens: &mut TokenStream2| self.expr.to_tokens(tokens);
            brace.surround(&mut tokens, f);
            return syn::parse2(tokens);
        }

        let expr = self.expr.clone();
        syn::parse2(expr.to_token_stream())
    }

    pub fn from_expr(expr: &Expr) -> Self {
        Self {
            brace: None,
            expr: expr.to_token_stream(),
        }
    }
    pub fn span(&self) -> proc_macro2::Span {
        if let Some(brace) = &self.brace {
            brace.span.span()
        } else {
            self.expr.span()
        }
    }
}

impl PartialEq for PartialExpr {
    fn eq(&self, other: &Self) -> bool {
        self.expr.to_string() == other.expr.to_string()
    }
}

impl Eq for PartialExpr {}

impl hash::Hash for PartialExpr {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.expr.to_string().hash(state);
    }
}
