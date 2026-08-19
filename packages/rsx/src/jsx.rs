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
//! templates, hot-reloading, and diagnostics work identically.

use crate::innerlude::*;
use proc_macro2::{Delimiter, Group, TokenStream as TokenStream2};
use syn::{
    Expr, Ident, LitBool, LitFloat, LitInt, LitStr, Token, braced,
    ext::IdentExt,
    parse::{Parse, ParseBuffer, ParseStream},
    punctuated::Punctuated,
    spanned::Spanned,
    token::Brace,
};

/// Parse a JSX/XML-like tag into a [`BodyNode`]. Expects the stream to be pointing at a `<` token.
pub(crate) fn parse_jsx_node(stream: ParseStream) -> syn::Result<BodyNode> {
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
        parse_element(stream)
    } else {
        parse_component(stream)
    }
}

fn parse_element(stream: ParseStream) -> syn::Result<BodyNode> {
    let name = stream.parse::<ElementName>()?;
    let (attributes, spreads) = parse_attributes(stream)?;

    let (brace, children) = parse_tag_end_and_children(stream, &name.to_string(), |close| {
        let close_name = close.parse::<ElementName>()?;
        if close_name != name {
            return Err(syn::Error::new(
                close_name.span(),
                format!("closing tag `</{close_name}>` does not match opening tag `<{name}>`"),
            ));
        }
        Ok(())
    })?;

    Ok(BodyNode::Element(Element::from_parts(
        name,
        attributes,
        spreads,
        children,
        Some(brace),
        Diagnostics::new(),
    )))
}

fn parse_component(stream: ParseStream) -> syn::Result<BodyNode> {
    let mut name = stream.parse::<syn::Path>()?;
    let generics = normalize_path(&mut name);

    let (fields, spreads) = parse_attributes(stream)?;

    let name_string = path_to_string(&name);
    let (brace, children) = parse_tag_end_and_children(stream, &name_string, |close| {
        let mut close_name = close.parse::<syn::Path>()?;
        normalize_path(&mut close_name);
        if path_to_string(&close_name) != name_string {
            return Err(syn::Error::new(
                close_name.span(),
                format!(
                    "closing tag `</{}>` does not match opening tag `<{}>`",
                    path_to_string(&close_name),
                    name_string
                ),
            ));
        }
        Ok(())
    })?;

    Ok(BodyNode::Component(Component::from_parts(
        name,
        generics,
        fields,
        spreads,
        children,
        Some(brace),
        Diagnostics::new(),
    )))
}

/// Parse the attributes of an open tag, stopping at `/>` or `>`
fn parse_attributes(stream: ParseStream) -> syn::Result<(Vec<Attribute>, Vec<Spread>)> {
    let mut attributes = Vec::new();
    let mut spreads = Vec::new();

    loop {
        if stream.peek(Token![/]) || stream.peek(Token![>]) {
            break;
        }

        if stream.is_empty() {
            return Err(stream.error("expected `>` or `/>` to close the tag"));
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
                dyn_idx: DynIdx::default(),
                comma: None,
            });
            continue;
        }

        // Attribute names are either string literals (custom attributes) or (dash-separated) idents
        let name = if stream.peek(LitStr) {
            AttributeName::Custom(stream.parse::<LitStr>()?)
        } else {
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
        };

        let value = if stream.peek(Token![=]) {
            stream.parse::<Token![=]>()?;

            if stream.peek(Brace) {
                // Braced expression values: `onclick={move |_| ...}`, `class={some_expr}`
                let content: ParseBuffer;
                braced!(content in stream);
                let value = AttributeValue::parse(&content)?;
                if !content.is_empty() {
                    return Err(content.error("unexpected tokens after attribute value"));
                }
                value
            } else if stream.peek(LitStr)
                || stream.peek(LitBool)
                || stream.peek(LitFloat)
                || stream.peek(LitInt)
            {
                // Literal values: `class="abc {def}"`, `width=100`
                AttributeValue::AttrLiteral(stream.parse::<HotLiteral>()?)
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
    }

    Ok((attributes, spreads))
}

/// Parse the end of an open tag (`>` or `/>`), children, and the closing tag if there is one
///
/// A brace token is synthesized from the span of the open tag's `>` so that consumers that expect
/// a braced body (completion hints, autofmt, ...) treat the tag as a complete node.
fn parse_tag_end_and_children(
    stream: ParseStream,
    name: &str,
    parse_close_name: impl FnOnce(ParseStream) -> syn::Result<()>,
) -> syn::Result<(Brace, Vec<BodyNode>)> {
    // Self-closing tag: `<div />`
    if stream.peek(Token![/]) {
        stream.parse::<Token![/]>()?;
        let gt = stream.parse::<Token![>]>()?;
        return Ok((synthetic_brace(gt), Vec::new()));
    }

    let gt = stream.parse::<Token![>]>()?;

    let mut children = Vec::new();
    loop {
        if stream.peek(Token![<]) && stream.peek2(Token![/]) {
            break;
        }

        if stream.is_empty() {
            return Err(syn::Error::new(
                gt.span(),
                format!("missing closing tag `</{name}>`"),
            ));
        }

        // Children of JSX tags are regular body nodes, so both syntaxes can be mixed freely
        children.push(stream.parse::<BodyNode>()?);
    }

    stream.parse::<Token![<]>()?;
    stream.parse::<Token![/]>()?;
    parse_close_name(stream)?;
    stream.parse::<Token![>]>()?;

    Ok((synthetic_brace(gt), children))
}

fn synthetic_brace(gt: Token![>]) -> Brace {
    let mut group = Group::new(Delimiter::Brace, TokenStream2::new());
    group.set_span(gt.span());
    Brace {
        span: group.delim_span(),
    }
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

        // Missing closing tag
        assert!(syn::parse2::<BodyNode>(quote! { <div>"hi" }).is_err());

        // Stray closing tag
        assert!(syn::parse2::<BodyNode>(quote! { </div> }).is_err());

        // Fragments are not supported
        assert!(syn::parse2::<BodyNode>(quote! { <>"hi"</> }).is_err());

        // Unbraced expression values
        assert!(syn::parse2::<BodyNode>(quote! { <div class=some_expr /> }).is_err());
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

        // Ensure codegen doesn't panic and produces a template
        let tokens = body.to_token_stream().to_string();
        assert!(tokens.contains("TemplateNode :: Element"));
    }
}
