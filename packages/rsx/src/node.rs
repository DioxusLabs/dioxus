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
        matches!(self, BodyNode::Text { .. })
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

        // if this is a dash-separated path, it's a web component (custom element)
        let body_stream = stream.fork();
        if let Ok(ElementName::Custom(name)) = body_stream.parse::<ElementName>() {
            if name.value().contains('-') && body_stream.peek(token::Brace) {
                return Ok(BodyNode::Element(stream.parse::<Element>()?));
            }
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
            return Ok(BodyNode::ForLoop(stream.parse()?));
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
                {
                    let ___nodes = (#exp).into_vnode(__cx);
                    ___nodes
                }
            }),
            BodyNode::ForLoop(exp) => {
                let ForLoop {
                    pat, expr, body, ..
                } = exp;

                let renderer: TemplateRenderer = TemplateRenderer {
                    roots: body,
                    location: None,
                };

                // Signals expose an issue with temporary lifetimes
                // We need to directly render out the nodes first to collapse their lifetime to <'a>
                // And then we can return them into the dyn loop
                tokens.append_all(quote! {
                    {
                        let ___nodes =(#expr).into_iter().map(|#pat| { #renderer }).into_vnode(__cx);
                        ___nodes
                    }
                })
            }
            BodyNode::IfChain(chain) => {
                if is_if_chain_terminated(chain) {
                    tokens.append_all(quote! {
                        {
                            let ___nodes = (#chain).into_vnode(__cx);
                            ___nodes
                        }
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
                        {
                            let ___nodes = (#body).into_vnode(__cx);
                            ___nodes
                        }
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
    pub brace_token: token::Brace,
}

impl Parse for ForLoop {
    fn parse(input: ParseStream) -> Result<Self> {
        let for_token: Token![for] = input.parse()?;

        let pat = Pat::parse_single(input)?;

        let in_token: Token![in] = input.parse()?;
        let expr: Expr = input.call(Expr::parse_without_eager_brace)?;

        let content;
        let brace_token = braced!(content in input);

        let mut children = vec![];

        while !content.is_empty() {
            children.push(content.parse()?);
        }

        Ok(Self {
            for_token,
            pat,
            in_token,
            body: children,
            expr: Box::new(expr),
            brace_token,
        })
    }
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
