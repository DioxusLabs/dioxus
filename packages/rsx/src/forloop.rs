use super::*;
use location::DynIdx;
use proc_macro2::TokenStream as TokenStream2;
use syn::{braced, Expr, Pat};

#[non_exhaustive]
#[derive(PartialEq, Eq, Clone, Debug)]
pub struct ForLoop {
    pub for_token: Token![for],
    pub pat: Pat,
    pub in_token: Token![in],
    pub expr: Box<Expr>,
    pub body: TemplateBody,
    pub dyn_idx: DynIdx,
}

impl Parse for ForLoop {
    fn parse(input: ParseStream) -> Result<Self> {
        // todo: better partial parsing
        // A bit stolen from `ExprForLoop` in the `syn` crate
        let for_token = input.parse()?;
        let pat = input.call(Pat::parse_single)?;
        let in_token = input.parse()?;
        let expr = input.call(Expr::parse_without_eager_brace)?;

        let content;
        let _brace = braced!(content in input);
        let body = content.parse()?;

        Ok(Self {
            for_token,
            pat,
            in_token,
            expr: Box::new(expr),
            body,
            dyn_idx: DynIdx::default(),
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
        });
    }
}

#[test]
fn parses_for_loop() {
    let toks = quote! {
        for item in 0..10 {
            div { "cool-{item}" }
            div { "cool-{item}" }
            div { "cool-{item}" }
        }
    };

    let for_loop: ForLoop = syn::parse2(toks).unwrap();
    assert!(for_loop.body.roots.len() == 3);
}
