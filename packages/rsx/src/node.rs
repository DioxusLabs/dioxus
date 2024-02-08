use super::*;

use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{quote, ToTokens, TokenStreamExt};
use syn::{
    braced,
    parse::{Parse, ParseStream},
    spanned::Spanned,
    token::{self, Brace},
    Expr, ExprIf, LitStr, Pat, Result,
};

/*
Parse
-> div {}
-> Component {}
-> component()
-> "text {with_args}"
-> {(0..10).map(|f| rsx!("asd"))}  // <--- notice the curly braces
*/
#[derive(PartialEq, Eq, Clone, Debug, Hash)]
pub enum BodyNode {
    Element(Element),
    Component(Component),
    ForLoop(ForLoop),
    IfChain(IfChain),
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

        // Match statements are special but have no special arm syntax
        // we could allow arm syntax if we wanted
        //
        // ```
        // match {
        //  val => div {}
        //  other_val => div {}
        // }
        // ```
        if stream.peek(Token![match]) {
            return Ok(BodyNode::RawExpr(stream.parse::<Expr>()?));
        }

        if stream.peek(token::Brace) {
            return Ok(BodyNode::RawExpr(stream.parse::<Expr>()?));
        }

        Err(syn::Error::new(
            stream.span(),
            "Expected a valid body node.\nExpressions must be wrapped in curly braces.",
        ))
    }
}

impl ToTokens for BodyNode {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        match &self {
            BodyNode::Element(_) => {
                unimplemented!("Elements are statically created in the template")
            }
            BodyNode::Component(comp) => comp.to_tokens(tokens),
            BodyNode::Text(txt) => tokens.append_all(quote! {
                dioxus_core::DynamicNode::Text(dioxus_core::VText::new(#txt.to_string()))
            }),
            BodyNode::RawExpr(exp) => tokens.append_all(quote! {
                {
                    let ___nodes = (#exp).into_dyn_node();
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
                        let ___nodes = (#expr).into_iter().map(|#pat| { #renderer }).into_dyn_node();
                        ___nodes
                    }
                })
            }
            BodyNode::IfChain(chain) => {
                let mut body = TokenStream2::new();
                let mut terminated = false;

                let mut elif = Some(chain);

                while let Some(chain) = elif {
                    let IfChain {
                        if_token,
                        cond,
                        then_branch,
                        else_if_branch,
                        else_branch,
                    } = chain;

                    let mut renderer: TemplateRenderer = TemplateRenderer {
                        roots: then_branch,
                        location: None,
                    };

                    body.append_all(quote! { #if_token #cond { Some({#renderer}) } });

                    if let Some(next) = else_if_branch {
                        body.append_all(quote! { else });
                        elif = Some(next);
                    } else if let Some(else_branch) = else_branch {
                        renderer.roots = else_branch;
                        body.append_all(quote! { else { Some({#renderer}) } });
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
                });
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

        let (brace_token, body) = parse_buffer_as_braced_children(input)?;

        Ok(Self {
            for_token,
            pat,
            in_token,
            body,
            brace_token,
            expr: Box::new(expr),
        })
    }
}

#[derive(PartialEq, Eq, Clone, Debug, Hash)]
pub struct IfChain {
    pub if_token: Token![if],
    pub cond: Box<Expr>,
    pub then_branch: Vec<BodyNode>,
    pub else_if_branch: Option<Box<IfChain>>,
    pub else_branch: Option<Vec<BodyNode>>,
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
        })
    }
}

fn parse_buffer_as_braced_children(
    input: &syn::parse::ParseBuffer<'_>,
) -> Result<(Brace, Vec<BodyNode>)> {
    let content;
    let brace_token = braced!(content in input);
    let mut then_branch = vec![];
    while !content.is_empty() {
        then_branch.push(content.parse()?);
    }
    Ok((brace_token, then_branch))
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
