use self::location::CallerLocation;

use super::*;

use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::{
    braced,
    spanned::Spanned,
    token::{self, Brace},
    Expr, ExprIf, LitStr, Pat,
};

mod attribute;
mod component;
mod element;
mod forloop;
mod ifchain;
mod text_node;

pub use attribute::*;
pub use component::*;
pub use element::*;
pub use forloop::*;
pub use ifchain::*;
pub use text_node::*;

#[derive(PartialEq, Eq, Clone, Debug, Hash)]
pub enum BodyNode {
    Element(Element),
    Text(TextNode),
    RawExpr(Expr),

    Component(Component),
    ForLoop(ForLoop),
    IfChain(IfChain),
}

impl BodyNode {
    pub fn is_litstr(&self) -> bool {
        matches!(self, BodyNode::Text { .. })
    }

    pub fn span(&self) -> Span {
        match self {
            BodyNode::Element(el) => el.name.span(),
            BodyNode::Component(component) => component.name.span(),
            BodyNode::Text(text) => text.input.source.span(),
            BodyNode::RawExpr(exp) => exp.span(),
            BodyNode::ForLoop(fl) => fl.for_token.span(),
            BodyNode::IfChain(f) => f.if_token.span(),
        }
    }

    pub(crate) fn set_location_idx(&self, idx: usize) {
        match self {
            BodyNode::IfChain(chain) => {
                chain.location.idx.set(idx);
            }
            BodyNode::ForLoop(floop) => {
                floop.location.idx.set(idx);
            }
            BodyNode::Component(comp) => {
                comp.location.idx.set(idx);
            }
            BodyNode::Text(text) => {
                text.location.idx.set(idx);
            }
            BodyNode::Element(_) => {}
            BodyNode::RawExpr(_) => {}
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
        match self {
            BodyNode::Element(_) => {
                unimplemented!("Elements are statically created in the template")
            }

            // Text is simple, just write it out
            BodyNode::Text(txt) => {
                let txt = &txt.input;
                if txt.is_static() {
                    tokens.append_all(quote! {
                        dioxus_core::DynamicNode::Text(dioxus_core::VText::new(#txt.to_string()))
                    })
                } else {
                    // If the text is dynamic, we actually create a signal of the formatted segments
                    // Crazy, right?
                    let segments = txt.as_htotreloaded();

                    let rendered_segments = txt.segments.iter().filter_map(|s| match s {
                        Segment::Literal(lit) => None,
                        Segment::Formatted(fmt) => {
                            // just render as a format_args! call
                            Some(quote! {
                                format!("{}", #fmt)
                            })
                        }
                    });

                    tokens.append_all(quote! {
                        dioxus_core::DynamicNode::Text(dioxus_core::VText::new({
                            // Create a signal of the formatted segments
                            // htotreloading will find this via its location and then update the signal
                            static __SIGNAL: GlobalSignal<FmtedSegments> = GlobalSignal::with_key(|| #segments, "__FMTBLOCK");

                            // render the signal and subscribe the component to its changes
                            __SIGNAL.with(|s| s.render_with(
                                vec![ #(#rendered_segments),* ]
                            ))
                        }))
                    })
                }
            }

            // Expressons too
            BodyNode::RawExpr(exp) => tokens.append_all(quote! {
                {
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

pub(crate) fn parse_buffer_as_braced_children(
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
