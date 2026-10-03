//! JSX/XML-like syntax for the rsx! macro
//!
//! The rsx! macro also accepts a JSX/XML-like tag syntax which can be freely mixed with the
//! regular block-based syntax. A `<` token switches the parser into JSX mode:
//!
//! ```rust, ignore
//! rsx! {
//!     <div class="container" onclick={move |_| println!("clicked")}>
//!         <h1>"Hello, world!"</h1>
//!         <img src="image.png" />
//!         <MyComponent prop="value">"children"</MyComponent>
//!
//!         // The regular syntax can be used inside JSX children (and vice versa)
//!         div { class: "inner", "More content" }
//!         for item in items {
//!             <span>"{item}"</span>
//!         }
//!     </div>
//! }
//! ```
//!
//! The syntax follows the regular rsx! rules:
//! - Text nodes are quoted string literals (`<h1>"Hello"</h1>`)
//! - Attribute values are literals (`class="abc"`) or braced expressions (`onclick={move |_| ...}`)
//! - Shorthand attributes are supported (`<div class />` is `class: class`)
//! - Spread attributes use `{..props}`
//!
//! The parsed result is the same [`Element`]/[`Component`] AST as the regular syntax, so
//! templates, hot-reloading, and diagnostics work identically. The tokens of the tags are kept
//! in the node's [`NodeDelimiter`].
//!
//! Like the regular syntax, tags that are still being typed (`<div cl`, `<div>` without a
//! closing tag) are parsed as far as possible and reported with a diagnostic instead of failing
//! the whole macro, so that completions keep working.

use crate::innerlude::*;
use proc_macro2::{Span, TokenStream as TokenStream2};
use proc_macro2_diagnostics::SpanDiagnosticExt;
use syn::{
    Expr, Ident, LitBool, LitFloat, LitInt, LitStr, Token, braced,
    ext::IdentExt,
    parse::{ParseBuffer, ParseStream, discouraged::Speculative},
    punctuated::Punctuated,
    spanned::Spanned,
    token::Brace,
};

/// Parse a JSX/XML-like tag into a [`BodyNode`]. Expects the stream to be pointing at a `<` token.
pub(crate) fn parse_jsx_node(stream: ParseStream) -> syn::Result<BodyNode> {
    parse_tag(stream, &mut Vec::new())
}

/// Parse a tag
///
/// `open_tags` are the names of the tags that this tag is directly nested in. A closing tag that
/// matches one of them means that this tag is missing its own closing tag, as opposed to its
/// closing tag being misspelled.
fn parse_tag(stream: ParseStream, open_tags: &mut Vec<String>) -> syn::Result<BodyNode> {
    let lt = stream.parse::<Token![<]>()?;

    if stream.peek(Token![/]) {
        return Err(syn::Error::new(
            lt.span,
            "encountered a closing tag without a matching opening tag",
        ));
    }

    if stream.peek(Token![>]) {
        return Err(syn::Error::new(
            lt.span,
            "fragments (`<>`) are not supported - list the children directly instead",
        ));
    }

    if !stream.peek(Ident::peek_any) && !stream.peek(Token![::]) {
        return Err(syn::Error::new(lt.span, "expected a tag name after `<`"));
    }

    // Decide between an element and a component using the same rules as the regular syntax:
    // - idents followed by a dash are web components
    // - a single lowercase ident with no underscores is an element
    // - everything else is a component
    let is_element = if stream.peek(Ident::peek_any) && stream.peek2(Token![-]) {
        true
    } else if stream.peek(Ident::peek_any) && !stream.peek2(Token![::]) {
        let ident = parse_raw_ident(&stream.fork())?;
        let name = ident.to_string();
        name.chars().next().unwrap().is_ascii_lowercase() && !name.contains('_')
    } else {
        false
    };

    if is_element {
        parse_element(stream, lt, open_tags)
    } else {
        parse_component(stream, lt, open_tags)
    }
}

fn parse_element(
    stream: ParseStream,
    lt: Token![<],
    open_tags: &mut Vec<String>,
) -> syn::Result<BodyNode> {
    let name = stream.parse::<ElementName>()?;
    let tag = TagName {
        name: name.to_string(),
        span: name.span(),
        parse: |stream| {
            let name = stream.parse::<ElementName>()?;
            Ok((name.to_string(), name.span()))
        },
    };

    let mut diagnostics = Diagnostics::new();
    let (attributes, spreads, end) = parse_open_tag(stream, &tag, &mut diagnostics)?;
    let (delimiter, children) = parse_tag_body(stream, lt, end, &tag, open_tags, &mut diagnostics)?;

    Ok(BodyNode::Element(Element::from_parts(
        name,
        attributes,
        spreads,
        children,
        NodeDelimiter::Tag(delimiter),
        diagnostics,
    )))
}

fn parse_component(
    stream: ParseStream,
    lt: Token![<],
    open_tags: &mut Vec<String>,
) -> syn::Result<BodyNode> {
    let mut name = stream.parse::<syn::Path>()?;
    let generics = normalize_path(&mut name);
    let tag = TagName {
        name: path_to_string(&name),
        span: name.span(),
        // The generics of a component don't need to be repeated in its closing tag
        parse: |stream| {
            let mut name = stream.parse::<syn::Path>()?;
            normalize_path(&mut name);
            Ok((path_to_string(&name), name.span()))
        },
    };

    let mut diagnostics = Diagnostics::new();
    let (fields, spreads, end) = parse_open_tag(stream, &tag, &mut diagnostics)?;
    let (delimiter, children) = parse_tag_body(stream, lt, end, &tag, open_tags, &mut diagnostics)?;

    Ok(BodyNode::Component(Component::from_parts(
        name,
        generics,
        fields,
        spreads,
        children,
        NodeDelimiter::Tag(delimiter),
        diagnostics,
    )))
}

/// The name of the tag that is being parsed
struct TagName {
    name: String,
    span: Span,
    /// Parse the name of a closing tag of the same kind (element or component)
    parse: fn(ParseStream) -> syn::Result<(String, Span)>,
}

/// How an open tag ended
enum OpenTagEnd {
    /// `<div />`
    SelfClosing(Token![/], Token![>]),
    /// `<div>`
    Open(Token![>]),
    /// The tag is incomplete: `<div class="a"`. A diagnostic has been emitted.
    Unterminated,
}

/// Parse the attributes of an open tag, up to and including its `/>` or `>`
///
/// Open tags that are still being typed (`<div cl`) are parsed as far as possible and reported
/// with a diagnostic rather than an error so that completions keep working.
fn parse_open_tag(
    stream: ParseStream,
    tag: &TagName,
    diagnostics: &mut Diagnostics,
) -> syn::Result<(Vec<Attribute>, Vec<Spread>, OpenTagEnd)> {
    let mut attributes = Vec::new();
    let mut spreads = Vec::new();

    let end = loop {
        if stream.peek(Token![>]) {
            break OpenTagEnd::Open(stream.parse()?);
        }

        if stream.peek(Token![/]) && stream.peek2(Token![>]) {
            break OpenTagEnd::SelfClosing(stream.parse()?, stream.parse()?);
        }

        // The start of another tag or the end of the input means this tag was never closed
        if stream.is_empty() || stream.peek(Token![<]) {
            diagnostics.push(tag.span.error(format!(
                "expected `>` or `/>` to close the `<{}` tag",
                tag.name
            )));
            break OpenTagEnd::Unterminated;
        }

        // Spread attributes: `{..expr}`
        if stream.peek(Brace) {
            let content: ParseBuffer;
            braced!(content in stream);
            let dots = content.parse::<Token![..]>().map_err(|_| {
                syn::Error::new(
                    content.span(),
                    "expected a spread attribute (`{..expr}`) - other braced expressions are not valid in a tag",
                )
            })?;
            let expr = content.parse::<Expr>()?;
            spreads.push(Spread {
                dots,
                expr,
                comma: None,
            });
            continue;
        }

        // Attribute names are either string literals (custom attributes) or (dash-separated) idents
        let name = if stream.peek(LitStr) {
            AttributeName::Custom(stream.parse::<LitStr>()?)
        } else if stream.peek(Ident::peek_any) {
            let raw = Punctuated::<Ident, Token![-]>::parse_separated_nonempty_with(
                stream,
                parse_raw_ident,
            )?;
            if raw.len() == 1 {
                AttributeName::BuiltIn(raw.into_iter().next().unwrap())
            } else {
                let span = raw.span();
                let name = raw
                    .into_iter()
                    .map(|ident| ident.to_string())
                    .collect::<Vec<_>>()
                    .join("-");
                AttributeName::Custom(LitStr::new(&name, span))
            }
        } else {
            return Err(stream.error("expected an attribute name, `>` or `/>`"));
        };

        let value = if stream.peek(Token![=]) {
            let eq = stream.parse::<Token![=]>()?;

            if stream.peek(Brace) {
                // Braced expression values: `onclick={move |_| ...}`, `class={some_expr}`
                parse_braced_value(stream)?
            } else if stream.peek(LitStr)
                || stream.peek(LitBool)
                || stream.peek(LitFloat)
                || stream.peek(LitInt)
            {
                // Literal values: `class="abc {def}"`, `width=100`
                AttributeValue::AttrLiteral(stream.parse::<HotLiteral>()?)
            } else if stream.is_empty() || stream.peek(Token![<]) {
                // The value hasn't been typed yet
                diagnostics.push(
                    eq.span
                        .error(format!("expected a value for the `{name}` attribute")),
                );
                break OpenTagEnd::Unterminated;
            } else {
                return Err(stream.error(
                    "attribute values must be literals or expressions wrapped in braces (`attr={expr}`)",
                ));
            }
        } else {
            // Shorthand attributes: `<div class>` is equivalent to `div { class }`
            match &name {
                AttributeName::BuiltIn(ident) => AttributeValue::Shorthand(ident.clone()),
                _ => {
                    return Err(syn::Error::new(
                        name.span(),
                        "custom attributes must have a value",
                    ));
                }
            }
        };

        let mut attribute = Attribute::from_raw(name, value);

        // Attributes in tags don't have commas, but stray ones are accepted for
        // compatibility with the regular syntax
        attribute.comma = stream.parse::<Token![,]>().ok();

        attributes.push(attribute);
    };

    Ok((attributes, spreads, end))
}

/// Parse a braced attribute value: `{expr}`
///
/// Expressions that don't parse (yet) are kept as raw tokens, like braced expressions in the
/// regular syntax, so that the macro still expands and completions work inside of them.
fn parse_braced_value(stream: ParseStream) -> syn::Result<AttributeValue> {
    let content: ParseBuffer;
    let brace = braced!(content in stream);

    let fork = content.fork();
    if let Ok(value) = fork.parse::<AttributeValue>()
        && fork.is_empty()
    {
        content.advance_to(&fork);
        return Ok(value);
    }

    let tokens = content.parse::<TokenStream2>()?;
    Ok(AttributeValue::AttrExpr(PartialExpr::from_braced(
        brace, tokens,
    )))
}

/// Parse the children and closing tag that follow an open tag
///
/// A missing closing tag is reported with a diagnostic rather than an error so that completions
/// keep working while the tag's children are being typed.
fn parse_tag_body(
    stream: ParseStream,
    lt: Token![<],
    end: OpenTagEnd,
    tag: &TagName,
    open_tags: &mut Vec<String>,
    diagnostics: &mut Diagnostics,
) -> syn::Result<(TagDelimiter, Vec<BodyNode>)> {
    let mut delimiter = TagDelimiter {
        lt,
        slash: None,
        gt: None,
        close: None,
    };

    match end {
        OpenTagEnd::Unterminated => return Ok((delimiter, Vec::new())),
        OpenTagEnd::SelfClosing(slash, gt) => {
            delimiter.slash = Some(slash);
            delimiter.gt = Some(gt);
            return Ok((delimiter, Vec::new()));
        }
        OpenTagEnd::Open(gt) => delimiter.gt = Some(gt),
    }

    let missing_closing_tag = || {
        tag.span
            .error(format!("missing closing tag `</{}>`", tag.name))
    };

    let mut children = Vec::new();
    loop {
        if stream.is_empty() {
            diagnostics.push(missing_closing_tag());
            break;
        }

        if stream.peek(Token![<]) && stream.peek2(Token![/]) {
            let fork = stream.fork();
            let close_lt = fork.parse::<Token![<]>()?;
            let close_slash = fork.parse::<Token![/]>()?;
            let (close_name, close_span) = (tag.parse)(&fork)?;

            if close_name == tag.name {
                stream.advance_to(&fork);
                delimiter.close = Some(ClosingTag {
                    lt: close_lt,
                    slash: close_slash,
                    gt: stream.parse()?,
                });
                break;
            }

            // The closing tag of a parent: this tag is the one that is missing its closing tag
            if open_tags.contains(&close_name) {
                diagnostics.push(missing_closing_tag());
                break;
            }

            return Err(syn::Error::new(
                close_span,
                format!(
                    "closing tag `</{close_name}>` does not match opening tag `<{}>`",
                    tag.name
                ),
            ));
        }

        // Children of tags are regular body nodes, so both syntaxes can be mixed freely
        if stream.peek(Token![<]) {
            open_tags.push(tag.name.clone());
            let child = parse_tag(stream, open_tags);
            open_tags.pop();
            children.push(child?);
        } else if let Some(child) = parse_unbraced_name(stream)? {
            children.push(child);
        } else {
            children.push(stream.parse::<BodyNode>()?);
        }
    }

    Ok((delimiter, children))
}

const UNQUOTED_TEXT: &str = "text in a tag must be a quoted string literal: `<h1>\"Hello\"</h1>`";

/// Catch text that was written without quotes, which would otherwise be parsed as the (invalid)
/// start of an element or component in the regular syntax and reported with a confusing error.
///
/// A single word directly followed by a tag (`<h1>Hello</h1>`) could also be an element or
/// component whose body hasn't been typed yet. It is parsed as that node, with a diagnostic, so
/// that completions keep working. Anything else that can't start a node is rejected.
fn parse_unbraced_name(stream: ParseStream) -> syn::Result<Option<BodyNode>> {
    // Text, expressions, control flow, and paths (`::Component {}`) are parsed as regular nodes
    if stream.peek(LitStr)
        || stream.peek(Brace)
        || stream.peek(Token![for])
        || stream.peek(Token![if])
        || stream.peek(Token![match])
        || stream.peek(Token![::])
    {
        return Ok(None);
    }

    if !stream.peek(Ident::peek_any) {
        return Err(stream.error(UNQUOTED_TEXT));
    }

    // Elements and components in the regular syntax: `div {}`, `my-element {}`, `module::Comp {}`
    if stream.peek2(Brace) || stream.peek2(Token![::]) || stream.peek2(Token![-]) {
        return Ok(None);
    }

    let fork = stream.fork();
    let ident = parse_raw_ident(&fork)?;
    if !fork.is_empty() && !fork.peek(Token![<]) {
        return Err(syn::Error::new(ident.span(), UNQUOTED_TEXT));
    }
    stream.advance_to(&fork);

    let name = ident.to_string();
    let is_element = name.chars().next().unwrap().is_ascii_lowercase() && !name.contains('_');

    let mut diagnostics = Diagnostics::new();
    diagnostics.push(
        ident
            .span()
            .error(format!("expected `{{` after `{name}`"))
            .help(format!(
                "elements and components must be followed by braces. If this is meant to be text, wrap it in quotes: `\"{name}\"`"
            )),
    );

    Ok(Some(if is_element {
        BodyNode::Element(Element::from_parts(
            ElementName::Ident(ident),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            NodeDelimiter::Missing,
            diagnostics,
        ))
    } else {
        BodyNode::Component(Component::from_parts(
            ident.into(),
            None,
            Vec::new(),
            Vec::new(),
            Vec::new(),
            NodeDelimiter::Missing,
            diagnostics,
        ))
    }))
}

fn path_to_string(path: &syn::Path) -> String {
    use quote::ToTokens;
    let mut name = path.to_token_stream().to_string();
    name.retain(|c| !c.is_whitespace());
    name
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;

    fn parse(input: proc_macro2::TokenStream) -> BodyNode {
        syn::parse2::<BodyNode>(input).unwrap()
    }

    #[test]
    fn parses_basic_element() {
        let node = parse(quote! { <div class="container">"Hello"</div> });
        let BodyNode::Element(el) = node else {
            panic!("expected element")
        };
        assert_eq!(el.name, "div");
        assert_eq!(el.raw_attributes.len(), 1);
        assert_eq!(el.raw_attributes[0].name.to_string(), "class");
        assert_eq!(el.children.len(), 1);
        assert!(el.diagnostics.is_empty());
    }

    #[test]
    fn parses_self_closing_element() {
        let node = parse(quote! { <img src="image.png" /> });
        let BodyNode::Element(el) = node else {
            panic!("expected element")
        };
        assert_eq!(el.name, "img");
        assert!(el.children.is_empty());
        assert!(el.diagnostics.is_empty());
    }

    #[test]
    fn parses_nested_elements() {
        let node = parse(quote! {
            <div>
                <h1>"Title"</h1>
                <p>"Body {text}"</p>
            </div>
        });
        let BodyNode::Element(el) = node else {
            panic!("expected element")
        };
        assert_eq!(el.children.len(), 2);
    }

    #[test]
    fn parses_component() {
        let node = parse(quote! { <MyComponent prop="value">"children"</MyComponent> });
        let BodyNode::Component(comp) = node else {
            panic!("expected component")
        };
        assert_eq!(comp.fields.len(), 1);
        assert_eq!(comp.children.roots.len(), 1);
        assert!(comp.diagnostics.is_empty());
    }

    #[test]
    fn parses_component_path_and_generics() {
        let node = parse(quote! { <some::cool::Component /> });
        assert!(matches!(node, BodyNode::Component(_)));

        let node = parse(quote! { <Outlet<R> /> });
        let BodyNode::Component(comp) = node else {
            panic!("expected component")
        };
        assert!(comp.generics.is_some());

        let node = parse(quote! { <Outlet<R>>"child"</Outlet<R>> });
        let BodyNode::Component(comp) = node else {
            panic!("expected component")
        };
        assert!(comp.generics.is_some());
        assert_eq!(comp.children.roots.len(), 1);
    }

    #[test]
    fn parses_web_component() {
        let node = parse(quote! { <my-web-component attr="value" /> });
        let BodyNode::Element(el) = node else {
            panic!("expected element")
        };
        assert!(matches!(el.name, ElementName::Custom(_)));
    }

    #[test]
    fn parses_event_handlers_and_expressions() {
        let node = parse(quote! {
            <button onclick={move |_| println!("clicked")} disabled={is_disabled}>
                "Click me"
            </button>
        });
        let BodyNode::Element(el) = node else {
            panic!("expected element")
        };
        assert_eq!(el.raw_attributes.len(), 2);
        assert!(matches!(
            el.raw_attributes[0].value,
            AttributeValue::EventTokens(_)
        ));
        assert!(matches!(
            el.raw_attributes[1].value,
            AttributeValue::AttrExpr(_)
        ));
    }

    #[test]
    fn parses_shorthand_and_custom_attributes() {
        let node = parse(quote! { <div class data-count="1" "custom-attr"="lit" /> });
        let BodyNode::Element(el) = node else {
            panic!("expected element")
        };
        assert_eq!(el.raw_attributes.len(), 3);
        assert!(matches!(
            el.raw_attributes[0].value,
            AttributeValue::Shorthand(_)
        ));
        assert_eq!(el.raw_attributes[1].name.to_string(), "data-count");
        assert_eq!(el.raw_attributes[2].name.to_string(), "custom-attr");
    }

    #[test]
    fn parses_spreads() {
        let node = parse(quote! { <div {..attrs} /> });
        let BodyNode::Element(el) = node else {
            panic!("expected element")
        };
        assert_eq!(el.spreads.len(), 1);

        let node = parse(quote! { <MyComponent {..props} /> });
        let BodyNode::Component(comp) = node else {
            panic!("expected component")
        };
        assert_eq!(comp.spreads.len(), 1);
    }

    #[test]
    fn mixes_syntax_styles() {
        // JSX children inside regular blocks
        let node = parse(quote! {
            div {
                class: "outer",
                <span>"inner"</span>
                p { "regular" }
            }
        });
        let BodyNode::Element(el) = node else {
            panic!("expected element")
        };
        assert_eq!(el.children.len(), 2);

        // Regular blocks, expressions, and control flow inside JSX children
        let node = parse(quote! {
            <div>
                p { class: "regular", "regular" }
                {some_expr}
                for item in items {
                    <span>"{item}"</span>
                }
                if cond {
                    <span>"conditional"</span>
                }
                <MyComponent />
            </div>
        });
        let BodyNode::Element(el) = node else {
            panic!("expected element")
        };
        assert_eq!(el.children.len(), 5);
    }

    #[test]
    fn merges_attributes() {
        let node = parse(quote! { <div class="foo" class="bar" /> });
        let BodyNode::Element(el) = node else {
            panic!("expected element")
        };
        assert_eq!(el.merged_attributes.len(), 1);
        assert!(el.diagnostics.is_empty());
    }

    #[test]
    fn rejects_invalid_input() {
        // Mismatched closing tag
        assert!(syn::parse2::<BodyNode>(quote! { <div>"hi"</span> }).is_err());
        assert!(syn::parse2::<BodyNode>(quote! { <MyComponent>"hi"</Other> }).is_err());

        // Stray closing tag
        assert!(syn::parse2::<BodyNode>(quote! { </div> }).is_err());

        // Fragments are not supported
        assert!(syn::parse2::<BodyNode>(quote! { <>"hi"</> }).is_err());

        // Unbraced expression values
        assert!(syn::parse2::<BodyNode>(quote! { <div class=some_expr /> }).is_err());
    }

    fn diagnostics(node: &BodyNode) -> &Diagnostics {
        match node {
            BodyNode::Element(el) => &el.diagnostics,
            BodyNode::Component(comp) => &comp.diagnostics,
            _ => panic!("expected an element or component"),
        }
    }

    #[test]
    fn keeps_tag_tokens() {
        let BodyNode::Element(el) = parse(quote! { <div>"hi"</div> }) else {
            panic!("expected element")
        };
        let tag = el.delimiter.tag().unwrap();
        assert!(tag.is_complete() && !tag.is_self_closing());

        let BodyNode::Component(comp) = parse(quote! { <MyComponent /> }) else {
            panic!("expected component")
        };
        let tag = comp.delimiter.tag().unwrap();
        assert!(tag.is_complete() && tag.is_self_closing());

        let BodyNode::Element(el) = parse(quote! { div { <span /> } }) else {
            panic!("expected element")
        };
        assert!(el.delimiter.brace().is_some());
    }

    #[test]
    fn closing_tags_dont_need_generics() {
        let node = parse(quote! { <Outlet<R>>"child"</Outlet> });
        assert!(diagnostics(&node).is_empty());
    }

    #[test]
    fn parses_expressions_before_tags() {
        // Braced expressions and `match` nodes end at a following tag
        let node = parse(quote! {
            <ul>
                {first}
                <li>"item"</li>
                match x { _ => rsx! {} }
                <li>"item"</li>
                match x { _ => rsx! {} }
            </ul>
        });
        let BodyNode::Element(el) = node else {
            panic!("expected element")
        };
        assert_eq!(el.children.len(), 5);

        // But braced attribute values in the regular syntax are still full expressions
        let node = parse(quote! { div { hidden: {a} < b } });
        let BodyNode::Element(el) = node else {
            panic!("expected element")
        };
        assert_eq!(el.raw_attributes.len(), 1);
        assert!(el.children.is_empty());
    }

    #[test]
    fn rejects_unquoted_text() {
        for input in [
            quote! { <h1>Hello world</h1> },
            quote! { <h1>Hello, world</h1> },
            quote! { <h1>42</h1> },
            quote! { <h1>"Hello" world!</h1> },
        ] {
            let err = syn::parse2::<BodyNode>(input).unwrap_err();
            assert!(err.to_string().contains("quoted string literal"), "{err}");
        }

        // A single word might also be an element or component that is still being typed
        for input in [quote! { <h1>Hello</h1> }, quote! { <h1>hello</h1> }] {
            let BodyNode::Element(el) = parse(input) else {
                panic!("expected element")
            };
            assert!(el.diagnostics.is_empty());
            assert_eq!(el.children.len(), 1);
            assert!(!diagnostics(&el.children[0]).is_empty());
        }
    }

    #[test]
    fn partially_expands_incomplete_tags() {
        // Unterminated open tags
        for input in [
            quote! { <di },
            quote! { <div cl },
            quote! { <div class="a" },
            quote! { <div class= },
            quote! { <MyComponent prop },
        ] {
            let node = parse(input);
            assert!(!diagnostics(&node).is_empty());
        }

        // Incomplete expressions in attribute values are kept as raw tokens
        let node = parse(quote! { <div class={foo.} /> });
        assert!(diagnostics(&node).is_empty());

        // Missing closing tags
        let node = parse(quote! { <div>"hi" });
        assert!(!diagnostics(&node).is_empty());

        // An unterminated tag doesn't swallow its siblings or the closing tag of its parent
        let BodyNode::Element(el) = parse(quote! { <div> <sp <b>"bold"</b> </div> }) else {
            panic!("expected element")
        };
        assert!(el.diagnostics.is_empty());
        assert_eq!(el.children.len(), 2);
        assert!(!diagnostics(&el.children[0]).is_empty());
        assert!(diagnostics(&el.children[1]).is_empty());

        // A tag that is missing its closing tag doesn't swallow the closing tag of its parent
        let BodyNode::Element(el) = parse(quote! { <div><span>"x"</div> }) else {
            panic!("expected element")
        };
        assert!(el.diagnostics.is_empty());
        assert!(el.delimiter.tag().unwrap().is_complete());
        assert!(!diagnostics(&el.children[0]).is_empty());

        // Incomplete tags inside of the regular syntax
        let BodyNode::Element(el) = parse(quote! { div { <sp } }) else {
            panic!("expected element")
        };
        assert!(!diagnostics(&el.children[0]).is_empty());
    }

    #[test]
    fn compiles_to_template() {
        use quote::ToTokens;

        let body: crate::CallBody = syn::parse2(quote! {
            <div class="container">
                <h1>"Hello, {name}!"</h1>
                <button onclick={move |_| println!("clicked")}>"Click"</button>
                <MyComponent prop="value" />
            </div>
        })
        .unwrap();

        let block: crate::CallBody = syn::parse2(quote! {
            div { class: "container",
                h1 { "Hello, {name}!" }
                button { onclick: move |_| println!("clicked"), "Click" }
                MyComponent { prop: "value" }
            }
        })
        .unwrap();

        // Tag syntax should generate exactly the same code as the equivalent block syntax
        assert_eq!(
            body.to_token_stream().to_string(),
            block.to_token_stream().to_string()
        );
    }
}
