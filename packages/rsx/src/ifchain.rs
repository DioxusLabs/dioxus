#[cfg(feature = "hot_reload")]
use dioxus_core::TemplateNode;

use crate::location::DynIdx;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use quote::{ToTokens, TokenStreamExt};
use syn::{
    parse::{Parse, ParseStream},
    Expr, ExprIf, Result, Token,
};

use crate::TemplateBody;

#[non_exhaustive]
#[derive(PartialEq, Eq, Clone, Debug)]
pub struct IfChain {
    pub if_token: Token![if],
    pub cond: Box<Expr>,
    pub then_branch: TemplateBody,
    pub else_if_branch: Option<Box<IfChain>>,
    pub else_branch: Option<TemplateBody>,
    pub dyn_idx: DynIdx,
}

impl IfChain {
    pub fn for_each_branch(&self, f: &mut impl FnMut(&TemplateBody)) {
        f(&self.then_branch);

        if let Some(else_if) = &self.else_if_branch {
            else_if.for_each_branch(f);
        }

        if let Some(else_branch) = &self.else_branch {
            f(else_branch);
        }
    }

    #[cfg(feature = "hot_reload")]
    pub fn to_template_node(&self) -> TemplateNode {
        TemplateNode::Dynamic {
            id: self.dyn_idx.get(),
        }
    }
}

impl Parse for IfChain {
    fn parse(input: ParseStream) -> Result<Self> {
        let if_token: Token![if] = input.parse()?;

        // stolen from ExprIf
        let cond = Box::new(input.call(Expr::parse_without_eager_brace)?);

        let content;
        syn::braced!(content in input);

        let then_branch = content.parse()?;

        let mut else_branch = None;
        let mut else_if_branch = None;

        // if the next token is `else`, set the else branch as the next if chain
        if input.peek(Token![else]) {
            input.parse::<Token![else]>()?;
            if input.peek(Token![if]) {
                else_if_branch = Some(Box::new(input.parse::<IfChain>()?));
            } else {
                let content;
                syn::braced!(content in input);
                else_branch = Some(content.parse()?);
            }
        }

        Ok(Self {
            cond,
            if_token,
            then_branch,
            else_if_branch,
            else_branch,
            dyn_idx: DynIdx::default(),
        })
    }
}

impl ToTokens for IfChain {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let mut body = TokenStream2::new();
        let mut terminated = false;

        let mut elif = Some(self);

        while let Some(chain) = elif {
            let IfChain {
                if_token,
                cond,
                then_branch,
                else_if_branch,
                else_branch,
                ..
            } = chain;

            body.append_all(quote! {
                #if_token #cond {
                    { #then_branch }
                }
            });

            if let Some(next) = else_if_branch {
                body.append_all(quote! {
                    else
                });
                elif = Some(next);
            } else if let Some(else_branch) = else_branch {
                body.append_all(quote! {
                    else {
                        {#else_branch}
                    }
                });
                terminated = true;
                break;
            } else {
                elif = None;
            }
        }

        if !terminated {
            body.append_all(quote! {
                else { dioxus_core::VNode::empty() }
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

pub(crate) fn is_if_chain_terminated(chain: &ExprIf) -> bool {
    let mut current = chain;
    loop {
        if let Some((_, else_block)) = &current.else_branch {
            if let Expr::If(else_if) = else_block.as_ref() {
                current = else_if;
            } else {
                return true;
            }
        } else {
            return false;
        }
    }
}

#[test]
fn parses_if_chain() {
    let input = quote! {
        if true {
            "one"
        } else if false {
            "two"
        } else {
            "three"
        }
    };

    let _chain: IfChain = syn::parse2(input).unwrap();
}
