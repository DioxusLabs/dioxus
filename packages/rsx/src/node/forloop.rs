use super::*;
#[non_exhaustive]
#[derive(PartialEq, Eq, Clone, Debug, Hash)]
pub struct ForLoop {
    pub for_token: Token![for],
    pub pat: Pat,
    pub in_token: Token![in],
    pub expr: Box<Expr>,
    pub body: Body,
    pub location: CallerLocation,
}

impl Parse for ForLoop {
    fn parse(input: ParseStream) -> Result<Self> {
        // A bit stolen from `ExprForLoop` in the `syn` crate
        let for_token: Token![for] = input.parse()?;
        let pat = Pat::parse_single(input)?;
        let in_token: Token![in] = input.parse()?;
        let expr: Expr = input.call(Expr::parse_without_eager_brace)?;

        let content;
        braced!(content in input);

        let body = content.parse()?;

        Ok(Self {
            for_token,
            pat,
            in_token,
            body,
            expr: Box::new(expr),
            location: CallerLocation::default(),
        })
    }
}

impl ToTokens for ForLoop {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let ForLoop {
            pat, expr, body, ..
        } = self;

        // the temporary is important so we create a lifetime binding
        tokens.append_all(quote! {
            {
                let ___nodes = (#expr).into_iter().map(|#pat| { #body }).into_dyn_node();
                ___nodes
            }
        })
    }
}
