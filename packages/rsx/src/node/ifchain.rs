use super::*;

#[non_exhaustive]
#[derive(PartialEq, Eq, Clone, Debug, Hash)]
pub struct IfChain {
    pub if_token: Token![if],
    pub cond: Box<Expr>,
    pub then_branch: Vec<BodyNode>,
    pub else_if_branch: Option<Box<IfChain>>,
    pub else_branch: Option<Vec<BodyNode>>,
    pub location: CallerLocation,
}

impl Parse for IfChain {
    fn parse(input: ParseStream) -> Result<Self> {
        let if_token: Token![if] = input.parse()?;

        // stolen from ExprIf
        let cond = Box::new(input.call(Expr::parse_without_eager_brace)?);

        let (_, then_branch) = parse_buffer_as_braced_children(input)?;

        let mut else_branch = None;
        let mut else_if_branch = None;

        // if the next token is `else`, set the else branch as the next if chain
        if input.peek(Token![else]) {
            input.parse::<Token![else]>()?;
            if input.peek(Token![if]) {
                else_if_branch = Some(Box::new(input.parse::<IfChain>()?));
            } else {
                let (_, else_branch_nodes) = parse_buffer_as_braced_children(input)?;
                else_branch = Some(else_branch_nodes);
            }
        }

        Ok(Self {
            cond,
            if_token,
            then_branch,
            else_if_branch,
            else_branch,
            location: CallerLocation::default(),
        })
    }
}

impl ToTokens for IfChain {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let mut body = TokenStream2::new();
        let mut terminated = false;

        let mut elif = Some(self);

        let base_idx = self.location.idx.get() * 1000;
        let mut cur_idx = base_idx + 1;

        while let Some(chain) = elif {
            let IfChain {
                if_token,
                cond,
                then_branch,
                else_if_branch,
                else_branch,
                ..
            } = chain;

            let renderer = TemplateRenderer::as_tokens_with_idx(then_branch, cur_idx);
            body.append_all(quote! { #if_token #cond { {#renderer} } });

            cur_idx += 1;

            if let Some(next) = else_if_branch {
                body.append_all(quote! { else });
                elif = Some(next);
            } else if let Some(else_branch) = else_branch {
                let renderer = TemplateRenderer::as_tokens_with_idx(else_branch, cur_idx);
                body.append_all(quote! { else { {#renderer} } });
                terminated = true;
                break;
            } else {
                elif = None;
            }
        }

        if !terminated {
            body.append_all(quote! {
                else { None }
            });
        }

        tokens.append_all(quote! {
            {
                let ___nodes = (#body).into_dyn_node();
                ___nodes
            }
        })
    }
}
