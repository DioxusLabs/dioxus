use proc_macro2::{TokenStream, TokenTree};
use proc_macro2_diagnostics::Diagnostic;
// use rstml::*;
use syn::{
    braced,
    parse::{discouraged::Speculative, ParseBuffer},
    punctuated::Punctuated,
    spanned::Spanned,
    token::{self, Brace, Colon, PathSep},
    Expr, ExprIf, Ident, LitStr, Pat, PathSegment, Result,
};
use syn::{parse::ParseStream, Token};

use crate::{location::CallerLocation, BodyNode, ElementName, ForLoop, IfChain};

/// A new stateful parser for rsx that stores span information and implicitly calculates the dynamic
/// context. This lets us do things like query a CallBody in place for its dynamic nodes, ifmts, and
/// other useful things we need IDs for.
///
/// We pool the templates into the top level Vec and then store any diagnostics (like recoverable errors)
/// onto this top struct.
///
/// This is a slightly more complex evolution of the previous design that segmented rsx into phases
/// (Callbody, DynamicContext, TemplateRenderer). We did this since lifetimes are an obvious way
/// of organizing that, but it required several tree walks that got confusing to manage.
///
/// This new approach combines all three of these by doing all the work at once, leading to more
/// centralized bookkepping and hopefully a simpler overall architecture in exchange for a more
/// complicated parser and ToTokens implementation.
///
/// I'd prefer to use a bump allocator or something like genbox to get around the self-referential issues
/// but have decided to work around the annoyances of Rust using traditional methods like Keys/Arenas.
#[derive(Debug)]
pub struct ParsedRsx {
    /// A single container of all the nodes in this rsx! call
    ///
    /// Subtemplates will be accessible through a separate architecture
    pub roots: Vec<BodyNode>,

    // All the diagnostics
    // The state of the parsed roots will be as good as we can get it, but we guarantee to emit diagnostics
    // This lets us properly expand the roots but also ensure the compile errors
    pub diagnostics: Vec<Diagnostic>,

    /// Hints that we're generating so the macro has better support for autocompletion with RA
    ///
    /// When we write these out, they'll be placed in a separate location and then we forward the
    /// cursor span to those tokens. This is a cute approach for providing autocompletion hints while
    /// also still generating proper code.
    ///
    /// We're bubbling these up into the top-level parser to make testing easier.
    pub completion_hints: Vec<()>,
}

impl syn::parse::Parse for ParsedRsx {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut parser = ParsedRsx::new();

        parser.parse_callbody(input);

        Ok(parser)
    }
}

/// A recoverable parser
///
/// We do our best to expand recursively if we can, but when processing linearly, we occasionally will
/// need to bail even though there is probably a *better* recoverable state to end up in.
impl ParsedRsx {
    fn new() -> Self {
        ParsedRsx {
            roots: vec![],
            diagnostics: vec![],
            completion_hints: vec![],
        }
    }

    fn parse_callbody(&mut self, input: ParseStream) {
        self.roots = self.parse_body_like(false, input);

        println!("is empty {}", input.is_empty());

        // If there's errors, handle them
        if !self.diagnostics.is_empty() {}
    }

    /// Parse each body node one by one
    /// Attempt to recover from certain cases so we can provide helpful partial expansion
    ///
    /// IE
    ///
    /// div {}
    /// h|
    /// div {}
    ///
    /// should partial expand but fail since we can tell that there's a missing curly.
    ///
    /// This will come after prop/attribute parsing which we always defer for completions when inserting
    /// the first element.
    fn parse_body_like(&mut self, expects_attrs: bool, input: ParseStream) -> Vec<BodyNode> {
        let mut roots = vec![];
        let mut attribute_likes: Vec<()> = vec![];

        // Are we parsing an open-ended block or a block confined to an element?

        // Process attributes, attempting some partial expansion
        if expects_attrs {
            todo!()
        }

        // Attempt to partially expand a few simple cases:
        //
        // adding a new element      -> h|
        // adding a new component    -> H|
        // incomplete exprs          -> {blah.}
        // attributes coming after elements
        // attributes intertwined with elements
        // mods with components      -> crate::icons::| {} (closing brace optional)
        //
        // These cases are generally *fine* but not the right syntax so we nip them
        //
        // This parser is now unfortunately pretty complex, sorry :/
        // nb: we defer to props-y things first
        while !input.is_empty() {
            // Peeking a litstr should never fail when parsing it, no diagnostics
            if input.peek(LitStr) {
                roots.push(BodyNode::Text(input.parse().unwrap()));
                continue;
            }

            // Are there any recoverable cases?
            // For now if parsing the for/expr stuff fails then just bail.
            // That way downstream parsing can fail
            //
            //
            // Transform for loops into into_iter calls
            if input.peek(Token![for]) {
                // self.push(&mut roots, self.parse_forloop(stream));
                continue;
            }

            // Transform unterminated if statements into terminated optional if statements
            if input.peek(Token![if]) {
                // return Ok(BodyNode::IfChain(stream.parse()?));
                continue;
            }

            // Match statements are special but have no special arm syntax
            // we could allow arm syntax if we wanted
            if input.peek(Token![match]) {
                // return Ok(BodyNode::RawExpr(stream.parse::<Expr>()?));
                continue;
            }

            // Raw expressions need to be wrapped in braces, but they don't need to be complete
            if input.peek(token::Brace) {
                fn parse_inline_expr(input: ParseStream) -> Result<TokenStream> {
                    let content: ParseBuffer;
                    let _brace_token = braced!(content in input);

                    let owned: TokenStream = content.parse().unwrap();

                    Ok(owned)
                }

                let raw_expr = crate::RawExpr {
                    expr: parse_inline_expr(input).unwrap(),
                };

                roots.push(BodyNode::RawExpr(raw_expr));

                continue;
            }

            // Okay all the special control flow is finished!
            // There's not much special about recoverably parsing those
            //
            // Let's attempt to recoverably parse compnents/elements
            // We're just gonna try and commit to parsing an ident here and then figure out
            // what should happen.

            // Parse a path sep then an ident
            // We're gonna build up the path manually if we have to.
            // This should at least give us decently recoverable states

            // Attempt components first by using path syntax
            if next_token_looks_like_component(&input) {
                let name = parse_partial_path(input).unwrap();

                if !input.peek(Brace) {
                    // Emit a diagnostic
                    // Emit an error
                    self.diagnostics.push(Diagnostic::spanned(
                        name.span(),
                        proc_macro2_diagnostics::Level::Error,
                        "Components must be followed by curly braces",
                    ));

                    roots.push(BodyNode::Component(crate::Component {
                        name,
                        prop_gen_args: None,
                        key: None,
                        fields: vec![],
                        manual_props: None,
                        location: Default::default(),
                        brace: Default::default(),
                        children: vec![],
                    }));

                    continue;
                }

                let (children, brace) = self.parse_braced_as_nodes(input);

                roots.push(BodyNode::Component(crate::Component {
                    name,
                    prop_gen_args: None,
                    key: None,
                    fields: vec![],
                    manual_props: None,
                    location: Default::default(),
                    brace,
                    children,
                }));

                continue;
            }

            // Attempt recoverable parsing of an element
            if input.peek(Ident) {
                let name = input.parse::<Ident>().unwrap();

                // If the next token is not a brace, then at least *try* to expand to a bad element
                if !input.peek(Brace) {
                    // Emit an error
                    self.diagnostics.push(Diagnostic::spanned(
                        name.span(),
                        proc_macro2_diagnostics::Level::Error,
                        "Elements must be followed by curly braces",
                    ));

                    // But then partially parse it using a made up brace token
                    let brace = Brace::default();
                    let children = vec![];
                    roots.push(BodyNode::Element(crate::Element {
                        name: ElementName::Ident(name),
                        key: None,
                        attributes: vec![],
                        merged_attributes: vec![],
                        brace,
                        children,
                    }));

                    // And break since this is a bad case
                    continue;
                }

                // Otherwise just parse
                let (children, brace) = self.parse_braced_as_nodes(input);

                roots.push(BodyNode::Element(crate::Element {
                    name: ElementName::Ident(name),
                    key: None,
                    attributes: vec![],
                    merged_attributes: vec![],
                    brace,
                    children,
                }));

                continue;
            }

            // // First, decide the parsing context we're going to use:
            // // 1: regular element
            // // 2: web-component (basically an element)
            // // 3: component (using the path)

            // // Attempt to parse an element
            // // This is the "perfect" case, so quick optimization
            // if stream.peek(Ident) && stream.peek2(Brace) {
            //     let ident = stream.fork().parse::<Ident>().unwrap();
            //     let el_name = ident.to_string();
            //     let first_char = el_name.chars().next().unwrap();

            //     if first_char.is_ascii_lowercase() && !el_name.contains('_') {
            //         let name: Ident = stream.parse().unwrap();
            //         let (children, brace) = self.parse_braced_as_nodes(stream);

            //         roots.push(BodyNode::Element(crate::Element {
            //             name: ElementName::Ident(name),
            //             key: None,
            //             attributes: vec![],
            //             merged_attributes: vec![],
            //             brace,
            //             children,
            //         }));

            //         continue;
            //     }
            // }

            // // If there's a path followed by a brace, then it's likely a component
            // let is_maybe_component_based_on_path_rules = {
            //     let forked = stream.fork();
            //     forked.parse::<syn::Path>().is_ok() && forked.peek(Brace)
            // };

            // // We're guaranteed to have a valid path and a brace is present in this fork
            // if is_maybe_component_based_on_path_rules {
            //     let name: syn::Path = stream.parse().unwrap();
            //     let (children, brace) = self.parse_braced_as_nodes(stream);

            //     roots.push(BodyNode::Component(crate::Component {
            //         name,
            //         prop_gen_args: None,
            //         key: None,
            //         fields: vec![],
            //         manual_props: None,
            //         location: Default::default(),
            //         brace,
            //         children,
            //     }));

            //     continue;
            // }

            // // If there's an ident immediately followed by a dash, it's a web component
            // // Web components support no namespacing, so just parse it as an element directly
            // if stream.peek(Ident) && stream.peek2(Token![-]) {
            //     // return Ok(BodyNode::Element(stream.parse::<Element>()?));
            //     continue;
            // }

            // Now... some recoverable cases we can attempt

            // Case 1:
            // If it's an ident followed by an ident, we can kinda tell the user is typing
            // We want to provide autocomplete for the element but also not compile
            // rsx! {
            //  d|
            //  div {}
            // }
            //
            // Also incldues this case:
            //
            // rsx! {
            //  div {
            //    div {}
            //    d|
            //    div {}
            //  }
            // }

            // Case 2:
            // If it's an ident followed by nothing, the user is typing at the end of the block
            // We want to provide autocomplete here too
            // rsx! { di| }

            // Case 3:
            // If it's an ident followed by a colon and nothing else, place the cursor into the attribute
            // block with a dummy token (maybe a ()?)
            // rsx! {
            //     div {
            //         id:|
            //     }
            // }

            // Case 4:
            // If we're inside an element or a component and writing the first ident, attempt to
            // provide property autocomplete
            // rsx! {
            //     div { cla| }
            // }

            // Ran into an unrecoverable case, just break and give a diagnostic on the next token
            panic!("Tokens remaining?");
        }

        roots
    }

    /// Parse element-like things, attempting to provide diagnostics
    fn parse_element_like(&mut self, input: ParseStream) {
        todo!()
    }

    fn parse_braced_as_nodes(&mut self, input: ParseStream) -> (Vec<BodyNode>, Brace) {
        match self.parse_braced_as_nodes_inner(input) {
            Ok(v) => v,
            Err(_) => {
                // insert a diagnostic
                (vec![], todo!())
            }
        }
    }

    /// Requires that
    fn parse_braced_as_nodes_inner(
        &mut self,
        input: ParseStream,
    ) -> Result<(Vec<BodyNode>, Brace)> {
        let content;
        let _brace_token = braced!(content in input);

        Ok((self.parse_body_like(false, &content), _brace_token))
    }

    fn parse_attribute_things(&mut self, stream: ParseStream) {}

    /// Parse starting from `for`.
    /// Attempt whatever recoverable tricks we can employ here
    ///
    /// Basically we want to bail if parsing the `for` structures fail, but bubble up any diagnostics
    /// from children since that's important for recovery.
    fn parse_forloop(&mut self, input: ParseStream) -> Result<ForLoop> {
        let for_token: Token![for] = input.parse()?;
        let pat = Pat::parse_single(input)?;
        let in_token: Token![in] = input.parse()?;
        let expr: Expr = input.call(Expr::parse_without_eager_brace)?;

        let body = self.parse_body_like(false, input);

        let floop = ForLoop {
            for_token,
            pat,
            in_token,
            body,
            expr: Box::new(expr),
            location: CallerLocation::default(),
        };

        Ok(floop)
    }

    /// Parse starting from `for`.
    /// Attempt whatever recoverable tricks we can employ here
    ///

    fn parse_ifchain(&mut self, input: ParseStream) -> Result<IfChain> {
        todo!()
    }
}

/// Try to partially expand poorly formed paths
///
/// ie some::cool::
///
///
fn next_is_partial_component_name(stream: &ParseStream) -> Option<syn::Path> {
    let blah = stream.parse::<syn::ExprPath>();
    todo!()
}

/// Build a path by parsing until we hit a curly brace or another ident
fn parse_partial_path(input: ParseStream) -> syn::Result<syn::Path> {
    let mut punctuated = Punctuated::new();

    loop {
        let segment = input.parse();

        let Ok(segment) = segment else {
            break;
        };

        punctuated.push_value(segment);

        if input.peek(PathSep) {
            punctuated.push_punct(input.parse()?);
        } else {
            break;
        }

        if input.is_empty() {
            break;
        }
    }

    Ok(syn::Path {
        leading_colon: None,
        segments: punctuated,
    })
}

fn next_token_looks_like_component(stream: &ParseStream) -> bool {
    // Crate-relative ::Blah syntax
    if stream.peek(PathSep) {
        return true;
    }

    // crate::blah
    if stream.peek(Ident) && stream.peek2(PathSep) {
        return true;
    }

    // Blah {}
    if let Ok(ident) = stream.fork().parse::<Ident>() {
        return ident.to_string().chars().next().unwrap().is_uppercase();
    };

    false
}
