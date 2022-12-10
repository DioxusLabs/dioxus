use std::marker::PhantomData;

use super::*;

use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{quote, ToTokens, TokenStreamExt};
use syn::{
    braced,
    parse::{Parse, ParseStream},
    spanned::Spanned,
    token, Expr, ExprIf, LitStr, Pat, Result,
};

/*
Parse
-> div {}
-> Component {}
-> component()
-> "text {with_args}"
-> (0..10).map(|f| rsx!("asd")),  // <--- notice the comma - must be a complete expr
*/
#[derive(PartialEq, Eq, Clone, Debug, Hash)]
pub enum BodyNode {
    Element(Element),
    Component(Component),
    ForLoop(ForLoop),
    IfChain(ExprIf),
    Text(IfmtInput),
    RawExpr(Expr),
}

impl BodyNode {
    pub fn is_litstr(&self) -> bool {
        matches!(self, BodyNode::Text(_))
    }

    pub fn span(&self) -> Span {
        match self {
            BodyNode::Element(el) => el.name.span(),
            BodyNode::Component(component) => component.name.span(),
            BodyNode::Text(text) => text.source.span(),
            BodyNode::RawExpr(exp) => exp.span(),
            BodyNode::ForLoop(fl) => fl.for_token.span(),
            BodyNode::IfChain(f) => f.if_token.span(),
        }
    }
}

impl Parse for BodyNode {
    fn parse(stream: ParseStream) -> Result<Self> {
        if stream.peek(LitStr) {
            return Ok(BodyNode::Text(stream.parse()?));
        }

        let body_stream = stream.fork();
        if let Ok(path) = body_stream.parse::<syn::Path>() {
            // this is an Element if path match of:
            // - one ident
            // - followed by `{`
            // - 1st char is lowercase
            // - no underscores (reserved for components)
            //
            // example:
            // div {}
            if let Some(ident) = path.get_ident() {
                let el_name = ident.to_string();

                let first_char = el_name.chars().next().unwrap();

                if body_stream.peek(token::Brace)
                    && first_char.is_ascii_lowercase()
                    && !el_name.contains('_')
                {
                    return Ok(BodyNode::Element(stream.parse::<Element>()?));
                }
            }

            // Otherwise this should be Component, allowed syntax:
            // - syn::Path
            // - PathArguments can only apper in last segment
            // - followed by `{` or `(`, note `(` cannot be used with one ident
            //
            // example
            // Div {}
            // ::Div {}
            // crate::Div {}
            // component {} <-- already handled by elements
            // ::component {}
            // crate::component{}
            // Input::<InputProps<'_, i32> {}
            // crate::Input::<InputProps<'_, i32> {}
            if body_stream.peek(token::Brace) {
                Component::validate_component_path(&path)?;
                return Ok(BodyNode::Component(stream.parse()?));
            }
        }

        // Transform for loops into into_iter calls
        if stream.peek(Token![for]) {
            let _f = stream.parse::<Token![for]>()?;
            let pat = stream.parse::<Pat>()?;
            let _i = stream.parse::<Token![in]>()?;
            let expr = stream.parse::<Box<Expr>>()?;

            let body;
            braced!(body in stream);
            let mut children = vec![];
            while !body.is_empty() {
                children.push(body.parse()?);
            }

            return Ok(BodyNode::ForLoop(ForLoop {
                for_token: _f,
                pat,
                in_token: _i,
                expr,
                body: children,
            }));
        }

        // Transform unterminated if statements into terminated optional if statements
        if stream.peek(Token![if]) {
            return Ok(BodyNode::IfChain(stream.parse()?));
        }

        Ok(BodyNode::RawExpr(stream.parse::<Expr>()?))
    }
}

impl ToTokens for BodyNode {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        match &self {
            BodyNode::Element(el) => el.to_tokens(tokens),
            BodyNode::Component(comp) => comp.to_tokens(tokens),
            BodyNode::Text(txt) => tokens.append_all(quote! {
                __cx.text_node(#txt)
            }),
            BodyNode::RawExpr(exp) => tokens.append_all(quote! {
                 __cx.make_node(#exp)
            }),
            BodyNode::ForLoop(exp) => {
                let ForLoop {
                    pat, expr, body, ..
                } = exp;

                let renderer: TemplateRenderer = TemplateRenderer {
                    roots: body,
                    phantom: PhantomData,
                };

                tokens.append_all(quote! {
                     __cx.make_node(
                        (#expr).into_iter().map(|#pat| { #renderer })
                     )
                })
            }
            BodyNode::IfChain(chain) => {
                if is_if_chain_terminated(chain) {
                    tokens.append_all(quote! {
                         __cx.make_node(#chain)
                    });
                } else {
                    let ExprIf {
                        cond,
                        then_branch,
                        else_branch,
                        ..
                    } = chain;

                    let mut body = TokenStream2::new();

                    body.append_all(quote! {
                        if #cond {
                            Some(#then_branch)
                        }
                    });

                    let mut elif = else_branch;

                    while let Some((_, ref branch)) = elif {
                        match branch.as_ref() {
                            Expr::If(ref eelif) => {
                                let ExprIf {
                                    cond,
                                    then_branch,
                                    else_branch,
                                    ..
                                } = eelif;

                                body.append_all(quote! {
                                    else if #cond {
                                        Some(#then_branch)
                                    }
                                });

                                elif = else_branch;
                            }
                            _ => {
                                body.append_all(quote! {
                                    else {
                                        #branch
                                    }
                                });
                                break;
                            }
                        }
                    }

                    body.append_all(quote! {
                        else { None }
                    });

                    tokens.append_all(quote! {
                        __cx.make_node(#body)
                    });
                }
            }
        }
    }
}

#[derive(PartialEq, Eq, Clone, Debug, Hash)]
pub struct ForLoop {
    pub for_token: Token![for],
    pub pat: Pat,
    pub in_token: Token![in],
    pub expr: Box<Expr>,
    pub body: Vec<BodyNode>,
}

fn is_if_chain_terminated(chain: &ExprIf) -> bool {
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
