use self::location::CallerLocation;

use super::*;

use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::{
    braced,
    parse::ParseBuffer,
    spanned::Spanned,
    token::{self, Brace},
    Expr, ExprCall, ExprIf, Ident, LitStr, Pat,
};

/*
Parse
-> div {}
-> Component {}
-> component()
-> "text {with_args}"
-> {(0..10).map(|f| rsx!("asd"))}  // <--- notice the curly braces
*/
#[derive(Clone, Debug)]
pub enum BodyNode {
    Element(Element),
    Text(IfmtInput),
    RawExpr(TokenStream2),
    Component(Component),
    ForLoop(ForLoop),
    IfChain(IfChain),
}

impl PartialEq for BodyNode {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Element(l), Self::Element(r)) => l == r,
            (Self::Text(l), Self::Text(r)) => l == r,
            (Self::RawExpr(l), Self::RawExpr(r)) => l.to_string() == r.to_string(),
            (Self::Component(l), Self::Component(r)) => l == r,
            (Self::ForLoop(l), Self::ForLoop(r)) => l == r,
            (Self::IfChain(l), Self::IfChain(r)) => l == r,
            _ => false,
        }
    }
}

impl Eq for BodyNode {}

impl Hash for BodyNode {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            Self::Element(el) => el.hash(state),
            Self::Text(text) => text.hash(state),
            Self::RawExpr(exp) => exp.to_string().hash(state),
            Self::Component(comp) => comp.hash(state),
            Self::ForLoop(for_loop) => for_loop.hash(state),
            Self::IfChain(if_chain) => if_chain.hash(state),
        }
    }
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

    pub(crate) fn parse_with_options(
        stream: ParseStream,
        partial_completions: bool,
    ) -> Result<Self> {
        // Make sure the next token is a brace if we're not in partial completion mode
        fn peek_brace(stream: &ParseBuffer, partial_completions: bool) -> bool {
            partial_completions || stream.peek(token::Brace)
        }

        if stream.peek(LitStr) {
            return Ok(BodyNode::Text(stream.parse()?));
        }

        // if this is a dash-separated path, it's a web component (custom element)
        let body_stream = stream.fork();
        if let Ok(ElementName::Custom(name)) = body_stream.parse::<ElementName>() {
            if name.value().contains('-') && peek_brace(&body_stream, partial_completions) {
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
                if peek_brace(&body_stream, partial_completions)
                    && !ident_looks_like_component(ident)
                {
                    return Ok(BodyNode::Element(Element::parse_with_options(
                        stream,
                        partial_completions,
                    )?));
                }
            }

            // If it is a single function call with a name that looks like a component, it should probably be a component
            // Eg, if we run into this:
            // ```rust
            // my_function(key, prop)
            // ```
            // We should tell the user that they need braces around props instead of turning the component call into an expression
            if let Ok(call) = stream.fork().parse::<ExprCall>() {
                if let Expr::Path(path) = call.func.as_ref() {
                    if let Some(ident) = path.path.get_ident() {
                        if ident_looks_like_component(ident) {
                            let function_args: Vec<_> = call
                                .args
                                .iter()
                                .map(|arg| arg.to_token_stream().to_string())
                                .collect();
                            let function_call = format!("{}({})", ident, function_args.join(", "));
                            let component_call = if function_args.is_empty() {
                                format!("{} {{}}", ident)
                            } else {
                                let component_args: Vec<_> = call
                                    .args
                                    .iter()
                                    .enumerate()
                                    .map(|(prop_count, arg)| {
                                        // Try to parse it as a shorthand field
                                        if let Ok(simple_ident) =
                                            syn::parse2::<Ident>(arg.to_token_stream())
                                        {
                                            format!("{}", simple_ident)
                                        } else {
                                            let ident = format!("prop{}", prop_count + 1);
                                            format!("{}: {}", ident, arg.to_token_stream())
                                        }
                                    })
                                    .collect();
                                format!("{} {{\n\t{}\n}}", ident, component_args.join(",\n\t"))
                            };
                            let error_text = format!(
                                "Expected a valid body node found a function call. Did you forget to add braces around props?\nComponents should be called with braces instead of being called as expressions.\nInstead of:\n```rust\n{function_call}\n```\nTry:\n```rust\n{component_call}\n```\nIf you are trying to call a function, not a component, you need to wrap your expression in braces.",
                            );
                            return Err(syn::Error::new(call.span(), error_text));
                        }
                    }
                }
            }

            // Otherwise this should be Component, allowed syntax:
            // - syn::Path
            // - PathArguments can only appear in last segment
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
            if peek_brace(&body_stream, partial_completions) {
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
            return Ok(BodyNode::RawExpr(stream.parse::<Expr>()?.to_token_stream()));
        }

        if stream.peek(token::Brace) {
            // If we are in strict mode, make sure thing inside the braces is actually a valid expression
            let combined = if !partial_completions {
                stream.parse::<Expr>()?.to_token_stream()
            } else {
                // otherwise, just take whatever is inside the braces. It might be invalid, but we still want to spit it out so we get completions
                let content;
                let brace = braced!(content in stream);
                let content: TokenStream2 = content.parse()?;
                let mut combined = TokenStream2::new();
                brace.surround(&mut combined, |inside_brace| {
                    inside_brace.append_all(content);
                });
                combined
            };

            return Ok(BodyNode::RawExpr(combined));
        }

        Err(syn::Error::new(
            stream.span(),
            "Expected a valid body node.\nExpressions must be wrapped in curly braces.",
        ))
    }
}

// Checks if an ident looks like a component
fn ident_looks_like_component(ident: &Ident) -> bool {
    let as_string = ident.to_string();
    let first_char = as_string.chars().next().unwrap();
    // Components either start with an uppercase letter or have an underscore in them
    first_char.is_ascii_uppercase() || as_string.contains('_')
}

impl Parse for BodyNode {
    fn parse(stream: ParseStream) -> Result<Self> {
        Self::parse_with_options(stream, true)
    }
}

impl ToTokens for BodyNode {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        match self {
            BodyNode::Element(_) => {
                unimplemented!("Elements are statically created in the template")
            }

            // Text is simple, just write it out
            BodyNode::Text(txt) => tokens.append_all(quote! {
                dioxus_core::DynamicNode::Text(dioxus_core::VText::new(#txt.to_string()))
            }),

            // Expressons too
            BodyNode::RawExpr(exp) => tokens.append_all(quote! {
                {
                    #[allow(clippy::let_and_return)]
                    let ___nodes = (#exp).into_dyn_node();
                    ___nodes
                }
            }),

            // todo:
            //
            // Component children should also participate in hotreloading
            // This is a *little* hard since components might not be able to take children in the
            // first place. I'm sure there's a hacky way to allow this... but it's not quite as
            // straightforward as a for loop.
            //
            // It might involve always generating a `children` field on the component and always
            // populating it with an empty template. This might lose the typesafety of whether
            // or not a component can even accept children - essentially allowing childrne in
            // every component - so it'd be breaking - but it would/could work.
            BodyNode::Component(comp) => tokens.append_all(quote! { #comp }),

            BodyNode::ForLoop(exp) => tokens.append_all(quote! { #exp }),

            BodyNode::IfChain(chain) => tokens.append_all(quote! { #chain }),
        }
    }
}

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

        let renderer = TemplateRenderer::as_tokens(body, None);

        // Signals expose an issue with temporary lifetimes
        // We need to directly render out the nodes first to collapse their lifetime to <'a>
        // And then we can return them into the dyn loop
        tokens.append_all(quote! {
            {
                #[allow(clippy::let_and_return)]
                let ___nodes = (#expr).into_iter().map(|#pat| { #renderer }).into_dyn_node();
                ___nodes
            }
        })
    }
}

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

        while let Some(chain) = elif {
            let IfChain {
                if_token,
                cond,
                then_branch,
                else_if_branch,
                else_branch,
                ..
            } = chain;

            let renderer = TemplateRenderer::as_tokens(then_branch, None);

            body.append_all(quote! { #if_token #cond { {#renderer} } });

            if let Some(next) = else_if_branch {
                body.append_all(quote! { else });
                elif = Some(next);
            } else if let Some(else_branch) = else_branch {
                let renderer = TemplateRenderer::as_tokens(else_branch, None);
                body.append_all(quote! { else { {#renderer} } });
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
                #[allow(clippy::let_and_return)]
                let ___nodes = (#body).into_dyn_node();
                ___nodes
            }
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
