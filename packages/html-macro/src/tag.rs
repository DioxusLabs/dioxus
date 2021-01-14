use proc_macro2::{Span, TokenStream, TokenTree};
use syn::parse::{Parse, ParseStream, Result};
use syn::spanned::Spanned;
use syn::token::Brace;
use syn::{braced, Block, Expr, Ident, Token};

/// The different kinds of tokens that we parse.
///
/// TODO: A better name than tag since not all of these are tags
#[derive(Debug)]
pub enum Tag {
    /// <div id="app" class=*CSS>
    /// <br />
    Open {
        name: Ident,
        attrs: Vec<Attr>,
        open_bracket_span: Span,
        closing_bracket_span: Span,
        is_self_closing: bool,
    },
    /// </div>
    Close {
        name: Ident,
        first_angle_bracket_span: Span,
    },
    /// html! { <div> Hello World </div> }
    ///
    ///  -> Hello world
    ///
    /// start_span -> the span for the first token within the text
    /// end_span -> the span for the last token within the text
    Text {
        text: String,
        start_span: Option<Span>,
        end_span: Option<Span>,
    },
    /// let text_var = VirtualNode::text("3");
    ///
    /// let iter_nodes =
    ///   vec![
    ///     html!{ <div></div> },
    ///     html! {<span> </span>}
    ///   ];
    ///
    /// html! {
    ///   <div>
    ///     Here are some examples of blocks
    ///     { text_var }
    ///     { iter_nodes }
    ///     { html! { <div> </div> }
    ///   </div>
    /// }
    Braced { block: Box<Block>, brace_span: Span },
}

/// The different kinds of tokens that we parse.
///
/// TODO: A better name than tag since not all of these are tags
#[derive(Debug, Eq, PartialEq)]
pub enum TagKind {
    Open,
    Close,
    Text,
    Braced,
}

/// id="my-id"
/// class="some classes"
/// etc...
#[derive(Debug)]
pub struct Attr {
    pub key: Ident,
    pub value: Expr,
}

impl Parse for Tag {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut input = input;

        // If it starts with a `<` it's either an open or close tag.
        //   ex: <div>
        //   ex: </em>
        if input.peek(Token![<]) {
            let first_angle_bracket_span = input.parse::<Token![<]>()?;
            let first_angle_bracket_span = first_angle_bracket_span.span();

            let optional_close: Option<Token![/]> = input.parse()?;
            let is_open_tag = optional_close.is_none();

            if is_open_tag {
                return parse_open_tag(&mut input, first_angle_bracket_span);
            } else {
                return parse_close_tag(&mut input, first_angle_bracket_span);
            }
        }

        // { node_inside_block }
        if input.peek(Brace) {
            return parse_block(&mut input);
        }

        return parse_text_node(&mut input);
    }
}

/// `<div id="app" class=*CSS>`
fn parse_open_tag(input: &mut ParseStream, open_bracket_span: Span) -> Result<Tag> {
    let name: Ident = input.parse()?;

    let attrs = parse_attributes(input)?;

    let is_self_closing: Option<Token![/]> = input.parse()?;
    let is_self_closing = is_self_closing.is_some();

    let closing_bracket = input.parse::<Token![>]>()?;
    let closing_bracket_span = closing_bracket.span();

    Ok(Tag::Open {
        name,
        attrs,
        open_bracket_span,
        closing_bracket_span,
        is_self_closing
    })
}

/// Parse the attributes starting from something like:
///     id="app" class=*CSS>
///
/// As soon as we see
///     >
/// We know that the element has no more attributes and our loop will end
fn parse_attributes(input: &mut ParseStream) -> Result<Vec<Attr>> {
    let mut attrs = Vec::new();

    // Do we see an identifier such as `id`? If so proceed
    while input.peek(Ident)
        || input.peek(Token![async])
        || input.peek(Token![for])
        || input.peek(Token![loop])
        || input.peek(Token![type])
    {
        // <link rel="stylesheet" type="text/css"
        //   .. async, for, loop, type need to be handled specially since they are keywords
        let maybe_async_key: Option<Token![async]> = input.parse()?;
        let maybe_for_key: Option<Token![for]> = input.parse()?;
        let maybe_loop_key: Option<Token![loop]> = input.parse()?;
        let maybe_type_key: Option<Token![type]> = input.parse()?;

        let key = if maybe_async_key.is_some() {
            Ident::new("async", maybe_async_key.unwrap().span())
        } else if maybe_for_key.is_some() {
            Ident::new("for", maybe_for_key.unwrap().span())
        } else if maybe_loop_key.is_some() {
            Ident::new("loop", maybe_loop_key.unwrap().span())
        } else if maybe_type_key.is_some() {
            Ident::new("type", maybe_type_key.unwrap().span())
        } else {
            input.parse()?
        };

        // =
        input.parse::<Token![=]>()?;

        // Continue parsing tokens until we see the next attribute or a closing > tag
        let mut value_tokens = TokenStream::new();

        loop {
            let tt: TokenTree = input.parse()?;
            value_tokens.extend(Some(tt));

            let has_attrib_key = input.peek(Ident)
                || input.peek(Token![async])
                || input.peek(Token![for])
                || input.peek(Token![loop])
                || input.peek(Token![type]);
            let peek_start_of_next_attr = has_attrib_key && input.peek2(Token![=]);

            let peek_end_of_tag = input.peek(Token![>]);

            let peek_self_closing = input.peek(Token![/]);

            if peek_end_of_tag || peek_start_of_next_attr || peek_self_closing {
                break;
            }
        }

        let value: Expr = syn::parse2(value_tokens)?;

        attrs.push(Attr { key, value });
    }

    Ok(attrs)
}

/// </div>
fn parse_close_tag(input: &mut ParseStream, first_angle_bracket_span: Span) -> Result<Tag> {
    let name: Ident = input.parse()?;

    input.parse::<Token![>]>()?;

    Ok(Tag::Close {
        name,
        first_angle_bracket_span,
    })
}

fn parse_block(input: &mut ParseStream) -> Result<Tag> {
    let content;
    let brace_token = braced!(content in input);

    let brace_span = brace_token.span;

    let block_expr = content.call(Block::parse_within)?;

    let block = Box::new(Block {
        brace_token,
        stmts: block_expr,
    });

    Ok(Tag::Braced { block, brace_span })
}

/// Parse a sequence of tokens until we run into a closing tag
///   html! { <div> Hello World </div> }
/// or a brace
///   html! { <div> Hello World { Braced } </div>
///
/// So, in the second case, there would be two VText nodes created. "Hello World" and "Braced".
///
/// Later in parser/text.rs we'll look at how close the VText nodes are to their neighboring tags
/// to determine whether or not to insert spacing.
///
/// So, in the examples above, since the opening "<div>" has a space after it we'll later transform
/// "Hello World" into " Hello World" in parser/tag.rs
fn parse_text_node(input: &mut ParseStream) -> Result<Tag> {
    // Continue parsing tokens until we see a closing tag <
    let _text_tokens = TokenStream::new();

    let mut text = "".to_string();

    let mut idx = 0;

    let mut start_span = None;

    let mut most_recent_span: Option<Span> = None;

    loop {
        if input.is_empty() {
            break;
        }

        let tt: TokenTree = input.parse()?;

        if idx == 0 {
            start_span = Some(tt.span());
            most_recent_span = Some(tt.span());
        }

        // TODO: Properly handle whitespace and new lines
        // https://github.com/chinedufn/percy/pull/97#discussion_r263039215
        if idx != 0 {
            if let Some(most_recent_span) = most_recent_span {
                let current_span_start = tt.span().start();
                let most_recent_span_end = most_recent_span.end();

                let spans_on_different_lines = current_span_start.line != most_recent_span_end.line;

                // Contraptions such as "Aren't" give the "'" and the "t" the
                // same span, even though they get parsed separately when calling
                // input.parse::<TokenTree>().
                // As in - it takes two input.parse calls to get the "'" and "t",
                // even though they have the same span.
                // This might be a bug in syn - but regardless we address this by
                // not inserting a space in this case.
                let span_comes_before_previous_span = current_span_start.column
                    < most_recent_span_end.column
                    && !spans_on_different_lines;

                // Spans are on different lines, insert space
                if spans_on_different_lines {
                    text += " ";
                } else if !span_comes_before_previous_span
                    && current_span_start.column - most_recent_span_end.column > 0
                {
                    text += " ";
                }
            }
        }

        text += &tt.to_string();

        most_recent_span = Some(tt.span());

        let peek_closing_tag = input.peek(Token![<]);
        let peek_start_block = input.peek(Brace);

        if peek_closing_tag || peek_start_block {
            break;
        }

        idx += 1;
    }

    Ok(Tag::Text {
        text,
        start_span,
        end_span: most_recent_span,
    })
}
