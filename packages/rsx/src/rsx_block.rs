//! An arbitrary block parser.
//!
//! Is meant to parse the contents of a block that is either a component or an element.
//! We put these together to cut down on code duplication and make the parsers a bit more resilient.
//!
//! This involves custom structs for name, attributes, and children, as well as a custom parser for the block itself.
//! It also bubbles out diagnostics if it can to give better errors.

use crate::innerlude::*;
use proc_macro2::Span;
use proc_macro2_diagnostics::SpanDiagnosticExt;
use syn::{
    ext::IdentExt,
    parse::{Parse, ParseBuffer, ParseStream},
    spanned::Spanned,
    token::{self, Brace},
    Expr, Ident, LitStr, Token,
};

/// An item in the form of
///
/// {
///  attributes,
///  ..spreads,
///  children
/// }
///
/// Does not make any guarantees about the contents of the block - this is meant to be verified by the
/// element/component impls themselves.
///
/// The name of the block is expected to be parsed by the parent parser. It will accept items out of
/// order if possible and then bubble up diagnostics to the parent. This lets us give better errors
/// and autocomplete
#[derive(PartialEq, Eq, Clone, Debug, Default)]
pub struct RsxBlock {
    pub brace: token::Brace,
    pub attributes: Vec<Attribute>,
    pub spreads: Vec<Spread>,
    pub children: Vec<BodyNode>,
    pub diagnostics: Diagnostics,
}

impl Parse for RsxBlock {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let content: ParseBuffer;
        let brace = syn::braced!(content in input);
        RsxBlock::parse_inner(&content, brace)
    }
}

impl RsxBlock {
    /// Only parse the children of the block - all others will be rejected
    pub fn parse_children(content: &ParseBuffer) -> syn::Result<Self> {
        let mut nodes = vec![];
        let mut diagnostics = Diagnostics::new();
        while !content.is_empty() {
            nodes.push(Self::parse_body_node_with_comma_diagnostics(
                content,
                &mut diagnostics,
            )?);
        }
        Ok(Self {
            children: nodes,
            diagnostics,
            ..Default::default()
        })
    }

    pub fn parse_inner(content: &ParseBuffer, brace: token::Brace) -> syn::Result<Self> {
        let mut items = vec![];
        let mut diagnostics = Diagnostics::new();

        // If we are after attributes, we can try to provide better completions and diagnostics
        // by parsing the following nodes as body nodes if they are ambiguous, we can parse them as body nodes
        let mut after_attributes = false;

        // Lots of manual parsing but it's important to do it all here to give the best diagnostics possible
        // We can do things like lookaheads, peeking, etc. to give better errors and autocomplete
        // We allow parsing in any order but complain if its done out of order.
        // Autofmt will fortunately fix this for us in most cases
        //
        // We do this by parsing the unambiguous cases first and then do some clever lookahead to parse the rest
        while !content.is_empty() {
            // Parse spread attributes
            if content.peek(Token![..]) {
                let dots = content.parse::<Token![..]>()?;

                // in case someone tries to do ...spread which is not valid
                if let Ok(extra) = content.parse::<Token![.]>() {
                    diagnostics.push(
                        extra
                            .span()
                            .error("Spread expressions only take two dots - not 3! (..spread)"),
                    );
                }

                let expr = content.parse::<Expr>()?;
                let attr = Spread {
                    expr,
                    dots,
                    dyn_idx: DynIdx::default(),
                    comma: content.parse().ok(),
                };

                if !content.is_empty() && attr.comma.is_none() {
                    diagnostics.push(
                        attr.span()
                            .error("Attributes must be separated by commas")
                            .help("Did you forget a comma?"),
                    );
                }
                items.push(RsxItem::Spread(attr));
                after_attributes = true;

                continue;
            }

            // Parse unambiguous attributes - these can't be confused with anything
            if (content.peek(LitStr) || content.peek(Ident::peek_any))
                && content.peek2(Token![:])
                && !content.peek3(Token![:])
            {
                let attr = content.parse::<Attribute>()?;

                if !content.is_empty() && attr.comma.is_none() {
                    diagnostics.push(
                        attr.span()
                            .error("Attributes must be separated by commas")
                            .help("Did you forget a comma?"),
                    );
                }

                items.push(RsxItem::Attribute(attr));

                continue;
            }

            // Eagerly match on completed children, generally
            if content.peek(LitStr)
                | content.peek(Token![for])
                | content.peek(Token![if])
                | content.peek(Token![match])
                | content.peek(token::Brace)
                // web components
                | (content.peek(Ident::peek_any) && content.peek2(Token![-]))
                // elements
                | (content.peek(Ident::peek_any) && (after_attributes || content.peek2(token::Brace)))
                // components
                | (content.peek(Ident::peek_any) && (after_attributes || content.peek2(token::Brace) || content.peek2(Token![::])))
            {
                items.push(RsxItem::Child(
                    Self::parse_body_node_with_comma_diagnostics(content, &mut diagnostics)?,
                ));
                if !content.is_empty() && content.peek(Token![,]) {
                    let comma = content.parse::<Token![,]>()?;
                    diagnostics.push(
                        comma.span().warning(
                            "Elements and text nodes do not need to be separated by commas.",
                        ),
                    );
                }
                after_attributes = true;
                continue;
            }

            // Parse shorthand attributes
            // todo: this might cause complications with partial expansion... think more about the cases
            // where we can imagine expansion and what better diagnostics we can provide
            if Self::peek_lowercase_ident(&content)
                    && !content.peek2(Brace)
                    && !content.peek2(Token![:]) // regular attributes / components with generics
                    && !content.peek2(Token![-]) // web components
                    && !content.peek2(Token![<]) // generics on components
                    // generics on components
                    && !content.peek2(Token![::])
            {
                let attribute = content.parse::<Attribute>()?;

                if !content.is_empty() && attribute.comma.is_none() {
                    diagnostics.push(
                        attribute
                            .span()
                            .error("Attributes must be separated by commas")
                            .help("Did you forget a comma?"),
                    );
                }

                items.push(RsxItem::Attribute(attribute));

                continue;
            }

            // Finally just attempt a bodynode parse
            items.push(RsxItem::Child(
                Self::parse_body_node_with_comma_diagnostics(content, &mut diagnostics)?,
            ))
        }

        // Validate the order of the items
        RsxBlock::validate(&items, &mut diagnostics);

        // todo: maybe make this a method such that the rsxblock is lossless
        // Decompose into attributes, spreads, and children
        let mut attributes = vec![];
        let mut spreads = vec![];
        let mut children = vec![];
        for item in items {
            match item {
                RsxItem::Attribute(attr) => attributes.push(attr),
                RsxItem::Spread(spread) => spreads.push(spread),
                RsxItem::Child(child) => children.push(child),
            }
        }

        Ok(Self {
            attributes,
            children,
            spreads,
            brace,
            diagnostics,
        })
    }

    // Parse a body node with diagnostics for unnecessary trailing commas
    fn parse_body_node_with_comma_diagnostics(
        content: &ParseBuffer,
        diagnostics: &mut Diagnostics,
    ) -> syn::Result<BodyNode> {
        let body_node = content.parse::<BodyNode>()?;
        if !content.is_empty() && content.peek(Token![,]) {
            let comma = content.parse::<Token![,]>()?;
            diagnostics.push(
                comma
                    .span()
                    .warning("Elements and text nodes do not need to be separated by commas."),
            );
        }
        Ok(body_node)
    }

    fn peek_lowercase_ident(stream: &ParseStream) -> bool {
        let Ok(ident) = stream.fork().call(Ident::parse_any) else {
            return false;
        };

        ident
            .to_string()
            .chars()
            .next()
            .unwrap()
            .is_ascii_lowercase()
    }

    /// Ensure the ordering of the items is correct
    /// - Attributes must come before children
    /// - Spreads must come before children
    /// - Spreads must come after attributes
    ///
    /// div {
    ///     key: "value",
    ///     ..props,
    ///     "Hello, world!"
    /// }
    fn validate(items: &[RsxItem], diagnostics: &mut Diagnostics) {
        #[derive(Debug, PartialEq, Eq)]
        enum ValidationState {
            Attributes,
            Spreads,
            Children,
        }
        use ValidationState::*;
        let mut state = ValidationState::Attributes;

        for item in items.iter() {
            match item {
                RsxItem::Attribute(_) => {
                    if state == Children || state == Spreads {
                        diagnostics.push(
                            item.span()
                                .error("Attributes must come before children in an element"),
                        );
                    }
                    state = Attributes;
                }
                RsxItem::Spread(_) => {
                    if state == Children {
                        diagnostics.push(
                            item.span()
                                .error("Spreads must come before children in an element"),
                        );
                    }
                    state = Spreads;
                }
                RsxItem::Child(_) => {
                    state = Children;
                }
            }
        }
    }
}

pub enum RsxItem {
    Attribute(Attribute),
    Spread(Spread),
    Child(BodyNode),
}

impl RsxItem {
    pub fn span(&self) -> Span {
        match self {
            RsxItem::Attribute(attr) => attr.span(),
            RsxItem::Spread(spread) => spread.dots.span(),
            RsxItem::Child(child) => child.span(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;

    #[test]
    fn basic_cases() {
        let input = quote! {
            { "Hello, world!" }
        };

        let block: RsxBlock = syn::parse2(input).unwrap();
        assert_eq!(block.attributes.len(), 0);
        assert_eq!(block.children.len(), 1);

        let input = quote! {
            {
                key: "value",
                onclick: move |_| {
                    "Hello, world!"
                },
                ..spread,
                "Hello, world!"
            }
        };

        let block: RsxBlock = syn::parse2(input).unwrap();
        dbg!(block);

        let complex_element = quote! {
            {
                key: "value",
                onclick2: move |_| {
                    "Hello, world!"
                },
                thing: if true { "value" },
                otherthing: if true { "value" } else { "value" },
                onclick: move |_| {
                    "Hello, world!"
                },
                ..spread,
                ..spread1
                ..spread2,
                "Hello, world!"
            }
        };

        let _block: RsxBlock = syn::parse2(complex_element).unwrap();

        let complex_component = quote! {
            {
                key: "value",
                onclick2: move |_| {
                    "Hello, world!"
                },
                ..spread,
                "Hello, world!"
            }
        };

        let _block: RsxBlock = syn::parse2(complex_component).unwrap();
    }

    /// Some tests of partial expansion to give better autocomplete
    #[test]
    fn partial_cases() {
        let with_handler = quote! {
            {
                onclick: move |_| {
                    some.
                }
            }
        };

        let _block: RsxBlock = syn::parse2(with_handler).unwrap();
    }

    /// Ensure the hotreload scoring algorithm works as expected
    #[test]
    fn hr_score() {
        let _block = quote! {
            {
                a: "value {cool}",
                b: "{cool} value",
                b: "{cool} {thing} value",
                b: "{thing} value",
            }
        };

        // loop { accumulate perfect matches }
        // stop when all matches are equally valid
        //
        // Remove new attr one by one as we find its perfect match. If it doesn't have a perfect match, we
        // score it instead.

        quote! {
            // start with
            div {
                div { class: "other {abc} {def} {hij}" } // 1, 1, 1
                div { class: "thing {abc} {def}" }       // 1, 1, 1
                // div { class: "thing {abc}" }             // 1, 0, 1
            }

            // end with
            div {
                h1 {
                    class: "thing {abc}" // 1, 1, MAX
                }
                h1 {
                    class: "thing {hij}" // 1, 1, MAX
                }
                // h2 {
                //     class: "thing {def}" // 1, 1, 0
                // }
                // h3 {
                //     class: "thing {def}" // 1, 1, 0
                // }
            }

            // how about shuffling components, for, if, etc
            Component {
                class: "thing {abc}",
                other: "other {abc} {def}",
            }
            Component {
                class: "thing",
                other: "other",
            }

            Component {
                class: "thing {abc}",
                other: "other",
            }
            Component {
                class: "thing {abc}",
                other: "other {abc} {def}",
            }
        };
    }

    #[test]
    fn kitchen_sink_parse() {
        let input = quote! {
            // Elements
            {
                class: "hello",
                id: "node-{node_id}",
                ..props,

                // Text Nodes
                "Hello, world!"

                // Exprs
                {rsx! { "hi again!" }}


                for item in 0..10 {
                    // "Second"
                    div { "cool-{item}" }
                }

                Link {
                    to: "/home",
                    class: "link {is_ready}",
                    "Home"
                }

                if false {
                    div { "hi again!?" }
                } else if true {
                    div { "its cool?" }
                } else {
                    div { "not nice !" }
                }
            }
        };

        let _parsed: RsxBlock = syn::parse2(input).unwrap();
    }

    #[test]
    fn simple_comp_syntax() {
        let input = quote! {
            { class: "inline-block mr-4", icons::icon_14 {} }
        };

        let _parsed: RsxBlock = syn::parse2(input).unwrap();
    }

    #[test]
    fn with_sutter() {
        let input = quote! {
            {
                div {}
                d
                div {}
            }
        };

        let _parsed: RsxBlock = syn::parse2(input).unwrap();
    }

    #[test]
    fn looks_like_prop_but_is_expr() {
        let input = quote! {
            {
                a: "asd".to_string(),
                // b can be omitted, and it will be filled with its default value
                c: "asd".to_string(),
                d: Some("asd".to_string()),
                e: Some("asd".to_string()),
            }
        };

        let _parsed: RsxBlock = syn::parse2(input).unwrap();
    }

    #[test]
    fn no_comma_diagnostics() {
        let input = quote! {
            { a, ..ComponentProps { a: 1, b: 2, c: 3, children: VNode::empty(), onclick: Default::default() } }
        };

        let parsed: RsxBlock = syn::parse2(input).unwrap();
        assert!(parsed.diagnostics.is_empty());
    }
    #[test]
    fn proper_attributes() {
        let input = quote! {
            {
                onclick: action,
                href,
                onmounted: onmounted,
                prevent_default,
                class,
                rel,
                target: tag_target,
                aria_current,
                ..attributes,
                {children}
            }
        };

        let parsed: RsxBlock = syn::parse2(input).unwrap();
        dbg!(parsed.attributes);
    }

    #[test]
    fn reserved_attributes() {
        let input = quote! {
            {
                label {
                    for: "blah",
                }
            }
        };

        let parsed: RsxBlock = syn::parse2(input).unwrap();
        dbg!(parsed.attributes);
    }

    #[test]
    fn diagnostics_check() {
        let input = quote::quote! {
            {
                class: "foo bar"
                "Hello world"
            }
        };

        let _parsed: RsxBlock = syn::parse2(input).unwrap();
    }

    #[test]
    fn incomplete_components() {
        let input = quote::quote! {
            {
                some::cool::Component
            }
        };

        let _parsed: RsxBlock = syn::parse2(input).unwrap();
    }

    #[test]
    fn incomplete_root_elements() {
        use syn::parse::Parser;

        let input = quote::quote! {
            di
        };

        let parsed = RsxBlock::parse_children.parse2(input).unwrap();
        let children = parsed.children;

        assert_eq!(children.len(), 1);
        if let BodyNode::Element(parsed) = &children[0] {
            assert_eq!(
                parsed.name,
                ElementName::Ident(Ident::new("di", Span::call_site()))
            );
        } else {
            panic!("expected element, got {:?}", children);
        }
        assert!(parsed.diagnostics.is_empty());
    }
}
