use super::*;
#[non_exhaustive]
#[derive(PartialEq, Eq, Clone, Debug, Hash)]
pub struct ForLoop {
    pub for_token: Token![for],
    pub pat: Pat,
    pub in_token: Token![in],
    pub expr: Box<Expr>,
    pub body: Vec<BodyNode>,
    pub brace_token: token::Brace,
    pub location: CallerLocation,
}

impl Parse for ForLoop {
    fn parse(input: ParseStream) -> Result<Self> {
        let for_token: Token![for] = input.parse()?;

        let pat = Pat::parse_single(input)?;

        let in_token: Token![in] = input.parse()?;
        let expr: Expr = input.call(Expr::parse_without_eager_brace)?;

        let (brace_token, body) = parse_buffer_as_braced_children(input)?;

        Ok(Self {
            for_token,
            pat,
            in_token,
            body,
            brace_token,
            location: CallerLocation::default(),
            expr: Box::new(expr),
        })
    }
}

impl ToTokens for ForLoop {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let ForLoop {
            pat, expr, body, ..
        } = self;

        let renderer = TemplateRenderer::as_tokens_with_idx(body, self.location.idx.get());

        // Signals expose an issue with temporary lifetimes
        // We need to directly render out the nodes first to collapse their lifetime to <'a>
        // And then we can return them into the dyn loop
        tokens.append_all(quote! {
            {
                let ___nodes = (#expr).into_iter().map(|#pat| { #renderer }).into_dyn_node();
                ___nodes
            }
        })
    }
}
