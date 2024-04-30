use super::*;
#[non_exhaustive]
#[derive(PartialEq, Eq, Clone, Debug, Hash)]
pub struct ForLoop {
    pub for_token: Token![for],
    pub pat: Pat,
    pub in_token: Token![in],
    pub expr: Box<Expr>,
    pub body: Vec<BodyNode>,
    pub location: CallerLocation,
}

impl Parse for ForLoop {
    fn parse(input: ParseStream) -> Result<Self> {
        // A bit stolen from `ExprForLoop` in the `syn` crate
        let for_token: Token![for] = input.parse()?;
        let pat = Pat::parse_single(input)?;
        let in_token: Token![in] = input.parse()?;
        let expr: Expr = input.call(Expr::parse_without_eager_brace)?;

        let (_brace_token, body) = parse_buffer_as_braced_children(input)?;

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
