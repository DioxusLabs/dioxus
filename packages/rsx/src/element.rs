use crate::innerlude::*;
use proc_macro2::{Span, TokenStream as TokenStream2};
use proc_macro2_diagnostics::SpanDiagnosticExt;
use quote::{quote, ToTokens, TokenStreamExt};
use std::fmt::{Display, Formatter};
use syn::{
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
    spanned::Spanned,
    token::Brace,
    Ident, LitStr, Result, Token,
};

/// Parse the VNode::Element type
#[derive(PartialEq, Eq, Clone, Debug)]
pub struct Element {
    /// div { } -> div
    pub name: ElementName,

    /// The actual attributes that were parsed
    pub raw_attributes: Vec<Attribute>,

    /// The attributes after merging - basically the formatted version of the combined attributes
    /// where possible.
    ///
    /// These are the actual attributes that get rendered out
    pub merged_attributes: Vec<Attribute>,

    /// The `...` spread attributes.
    pub spreads: Vec<Spread>,

    // /// Elements can have multiple, unlike components which can only have one
    // pub spreads: Vec<Spread>,
    /// The children of the element
    pub children: Vec<BodyNode>,

    /// the brace of the `div { }`
    pub brace: Option<Brace>,

    /// A list of diagnostics that were generated during parsing. This element might be a valid rsx_block
    /// but not technically a valid element - these diagnostics tell us what's wrong and then are used
    /// when rendering
    pub diagnostics: Diagnostics,
}

impl Parse for Element {
    fn parse(stream: ParseStream) -> Result<Self> {
        let name = stream.parse::<ElementName>()?;

        // We very liberally parse elements - they might not even have a brace!
        // This is designed such that we can throw a compile error but still give autocomplete
        // ... partial completions mean we do some weird parsing to get the right completions
        let mut brace = None;
        let mut block = RsxBlock::default();

        match stream.peek(Brace) {
            // If the element is followed by a brace, it is complete. Parse the body
            true => {
                block = stream.parse::<RsxBlock>()?;
                brace = Some(block.brace);
            }

            // Otherwise, it is incomplete. Add a diagnostic
            false => block.diagnostics.push(
                name.span()
                    .error("Elements must be followed by braces")
                    .help("Did you forget a brace?"),
            ),
        }

        // Make sure these attributes have an el_name set for completions and Template generation
        for attr in block.attributes.iter_mut() {
            attr.el_name = Some(name.clone());
        }

        // Assemble the new element from the contents of the block
        let mut element = Element {
            brace,
            name,
            raw_attributes: block.attributes,
            children: block.children,
            diagnostics: block.diagnostics,
            spreads: block.spreads.clone(),
            merged_attributes: Vec::new(),
        };

        // And then merge the various attributes together
        // The original raw_attributes are kept for lossless parsing used by hotreload/autofmt
        element.merge_attributes();

        // And then merge the spreads *after* the attributes are merged. This ensures walking the
        // merged attributes in path order stops before we hit the spreads, but spreads are still
        // counted as dynamic attributes
        for spread in block.spreads.iter() {
            element.merged_attributes.push(Attribute {
                name: AttributeName::Spread(spread.dots),
                colon: None,
                value: AttributeValue::AttrExpr(PartialExpr::from_expr(&spread.expr)),
                comma: spread.comma,
                dyn_idx: spread.dyn_idx.clone(),
                el_name: None,
            });
        }

        Ok(element)
    }
}

impl ToTokens for Element {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let el = self;
        let el_name = &el.name;

        let ns = |name| match el_name {
            ElementName::Ident(i) => quote! { dioxus_elements::#i::#name },
            ElementName::Custom(_) => quote! { None },
        };

        let static_attrs = el
            .merged_attributes
            .iter()
            .map(|attr| {
                // Rendering static attributes requires a bit more work than just a dynamic attrs
                // Early return for dynamic attributes
                let Some((name, value)) = attr.as_static_str_literal() else {
                    let id = attr.dyn_idx.get();
                    return quote! { dioxus_core::TemplateAttribute::Dynamic { id: #id  } };
                };

                let ns = match name {
                    AttributeName::BuiltIn(name) => ns(quote!(#name.1)),
                    AttributeName::Custom(_) => quote!(None),
                    AttributeName::Spread(_) => {
                        unreachable!("spread attributes should not be static")
                    }
                };

                let name = match (el_name, name) {
                    (ElementName::Ident(_), AttributeName::BuiltIn(_)) => {
                        quote! { dioxus_elements::#el_name::#name.0 }
                    }
                    //hmmmm I think we could just totokens this, but the to_string might be inserting quotes
                    _ => {
                        let as_string = name.to_string();
                        quote! { #as_string }
                    }
                };

                let value = value.to_static().unwrap();

                quote! {
                    dioxus_core::TemplateAttribute::Static {
                        name: #name,
                        namespace: #ns,
                        value: #value,
                    }
                }
            })
            .collect::<Vec<_>>();

        // Render either the child
        let children = el.children.iter().map(|c| match c {
            BodyNode::Element(el) => quote! { #el },
            BodyNode::Text(text) if text.is_static() => {
                let text = text.input.to_static().unwrap();
                quote! { dioxus_core::TemplateNode::Text { text: #text } }
            }
            BodyNode::Text(text) => {
                let id = text.dyn_idx.get();
                quote! { dioxus_core::TemplateNode::Dynamic { id: #id } }
            }
            BodyNode::ForLoop(floop) => {
                let id = floop.dyn_idx.get();
                quote! { dioxus_core::TemplateNode::Dynamic { id: #id } }
            }
            BodyNode::RawExpr(exp) => {
                let id = exp.dyn_idx.get();
                quote! { dioxus_core::TemplateNode::Dynamic { id: #id } }
            }
            BodyNode::Component(exp) => {
                let id = exp.dyn_idx.get();
                quote! { dioxus_core::TemplateNode::Dynamic { id: #id } }
            }
            BodyNode::IfChain(exp) => {
                let id = exp.dyn_idx.get();
                quote! { dioxus_core::TemplateNode::Dynamic { id: #id } }
            }
        });

        let ns = ns(quote!(NAME_SPACE));
        let el_name = el_name.tag_name();
        let diagnostics = &el.diagnostics;
        let completion_hints = &el.completion_hints();

        // todo: generate less code if there's no diagnostics by not including the curlies
        tokens.append_all(quote! {
            {
                #completion_hints

                #diagnostics

                dioxus_core::TemplateNode::Element {
                    tag: #el_name,
                    namespace: #ns,
                    attrs: &[ #(#static_attrs),* ],
                    children: &[ #(#children),* ],
                }
            }
        })
    }
}

impl Element {
    /// Collapses ifmt attributes into a single dynamic attribute using a space or `;` as a delimiter
    ///
    /// ```ignore,
    /// div {
    ///     class: "abc-def",
    ///     class: if some_expr { "abc" },
    /// }
    /// ```
    fn merge_attributes(&mut self) {
        let mut attrs: Vec<&Attribute> = vec![];

        for attr in &self.raw_attributes {
            if attrs.iter().any(|old_attr| old_attr.name == attr.name) {
                continue;
            }

            attrs.push(attr);
        }

        for attr in attrs {
            if attr.name.to_string() == "key" {
                continue;
            }

            // Collect all the attributes with the same name
            let matching_attrs = self
                .raw_attributes
                .iter()
                .filter(|a| a.name == attr.name)
                .collect::<Vec<_>>();

            // if there's only one attribute with this name, then we don't need to merge anything
            if matching_attrs.len() == 1 {
                self.merged_attributes.push(attr.clone());
                continue;
            }

            // If there are multiple attributes with the same name, then we need to merge them
            // This will be done by creating an ifmt attribute that combines all the segments
            // We might want to throw a diagnostic of trying to merge things together that might not
            // make a whole lot of sense - like merging two exprs together
            let mut out = IfmtInput::new(attr.span());

            for (idx, matching_attr) in matching_attrs.iter().enumerate() {
                // If this is the first attribute, then we don't need to add a delimiter
                if idx != 0 {
                    // FIXME: I don't want to special case anything - but our delimiter is special cased to a space
                    // We really don't want to special case anything in the macro, but the hope here is that
                    // multiline strings can be merged with a space
                    out.push_raw_str(" ".to_string());
                }

                // Merge raw literals into the output
                if let AttributeValue::AttrLiteral(lit) = &matching_attr.value {
                    if let HotLiteralType::Fmted(new) = &lit.value {
                        out.push_ifmt(new.clone());
                        continue;
                    }
                }

                // Merge `if cond { "abc" }` into the output
                if let AttributeValue::AttrOptionalExpr { condition, value } = &matching_attr.value
                {
                    if let AttributeValue::AttrLiteral(lit) = value.as_ref() {
                        if let HotLiteralType::Fmted(new) = &lit.value {
                            out.push_condition(condition.clone(), new.clone());
                            continue;
                        }
                    }
                }

                // unwind in case there's a test or two that cares about this weird state
                _ = out.segments.pop();
                self.diagnostics.push(matching_attr.span().error("Cannot merge non-fmt literals").help(
                    "Only formatted strings can be merged together. If you want to merge literals, you can use a format string.",
                ));
            }

            let out_lit = HotLiteral {
                value: HotLiteralType::Fmted(out),
                hr_idx: Default::default(),
            };

            self.merged_attributes.push(Attribute {
                name: attr.name.clone(),
                value: AttributeValue::AttrLiteral(out_lit),
                colon: attr.colon,
                dyn_idx: attr.dyn_idx.clone(),
                comma: matching_attrs.last().unwrap().comma,
                el_name: attr.el_name.clone(),
            });
        }
    }

    pub(crate) fn key(&self) -> Option<&IfmtInput> {
        for attr in &self.raw_attributes {
            if let AttributeName::BuiltIn(name) = &attr.name {
                if name == "key" {
                    return attr.ifmt();
                }
            }
        }

        None
    }

    fn completion_hints(&self) -> TokenStream2 {
        // If there is already a brace, we don't need any completion hints
        if self.brace.is_some() {
            return quote! {};
        }

        let ElementName::Ident(name) = &self.name else {
            return quote! {};
        };

        quote! {
            {
                #[allow(dead_code)]
                #[doc(hidden)]
                mod __completions {
                    fn ignore() {
                        super::dioxus_elements::elements::completions::CompleteWithBraces::#name
                    }
                }
            }
        }
    }
}

#[derive(PartialEq, Eq, Clone, Debug, Hash)]
pub enum ElementName {
    Ident(Ident),
    Custom(LitStr),
}

impl ToTokens for ElementName {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        match self {
            ElementName::Ident(i) => tokens.append_all(quote! { #i }),
            ElementName::Custom(s) => s.to_tokens(tokens),
        }
    }
}

impl Parse for ElementName {
    fn parse(stream: ParseStream) -> Result<Self> {
        let raw =
            Punctuated::<Ident, Token![-]>::parse_separated_nonempty_with(stream, parse_raw_ident)?;
        if raw.len() == 1 {
            Ok(ElementName::Ident(raw.into_iter().next().unwrap()))
        } else {
            let span = raw.span();
            let tag = raw
                .into_iter()
                .map(|ident| ident.to_string())
                .collect::<Vec<_>>()
                .join("-");
            let tag = LitStr::new(&tag, span);
            Ok(ElementName::Custom(tag))
        }
    }
}

impl ElementName {
    pub(crate) fn tag_name(&self) -> TokenStream2 {
        match self {
            ElementName::Ident(i) => quote! { dioxus_elements::elements::#i::TAG_NAME },
            ElementName::Custom(s) => quote! { #s },
        }
    }

    pub fn span(&self) -> Span {
        match self {
            ElementName::Ident(i) => i.span(),
            ElementName::Custom(s) => s.span(),
        }
    }
}

impl PartialEq<&str> for ElementName {
    fn eq(&self, other: &&str) -> bool {
        match self {
            ElementName::Ident(i) => i == *other,
            ElementName::Custom(s) => s.value() == *other,
        }
    }
}

impl Display for ElementName {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ElementName::Ident(i) => write!(f, "{}", i),
            ElementName::Custom(s) => write!(f, "{}", s.value()),
        }
    }
}

#[test]
fn parses_name() {
    let _parsed: ElementName = syn::parse2(quote::quote! { div }).unwrap();
    let _parsed: ElementName = syn::parse2(quote::quote! { some-cool-element }).unwrap();

    let _parsed: Element = syn::parse2(quote::quote! { div {} }).unwrap();
    let _parsed: Element = syn::parse2(quote::quote! { some-cool-element {} }).unwrap();

    let parsed: Element = syn::parse2(quote::quote! {
        some-cool-div {
            id: "hi",
            id: "hi {abc}",
            id: "hi {def}",
            class: 123,
            something: bool,
            data_attr: "data",
            data_attr: "data2",
            data_attr: "data3",
            exp: { some_expr },
            something: {cool},
            something: bool,
            something: 123,
            onclick: move |_| {
                println!("hello world");
            },
            "some-attr": "hello world",
            onclick: move |_| {},
            class: "hello world",
            id: "my-id",
            data_attr: "data",
            data_attr: "data2",
            data_attr: "data3",
            "somte_attr3": "hello world",
            something: {cool},
            something: bool,
            something: 123,
            onclick: move |_| {
                println!("hello world");
            },
            ..attrs1,
            ..attrs2,
            ..attrs3
        }
    })
    .unwrap();

    dbg!(parsed);
}

#[test]
fn parses_variety() {
    let input = quote::quote! {
        div {
            class: "hello world",
            id: "my-id",
            data_attr: "data",
            data_attr: "data2",
            data_attr: "data3",
            "somte_attr3": "hello world",
            something: {cool},
            something: bool,
            something: 123,
            onclick: move |_| {
                println!("hello world");
            },
            ..attrs,
            ..attrs2,
            ..attrs3
        }
    };

    let parsed: Element = syn::parse2(input).unwrap();
    dbg!(parsed);
}

#[test]
fn to_tokens_properly() {
    let input = quote::quote! {
        div {
            class: "hello world",
            class2: "hello {world}",
            class3: "goodbye {world}",
            class4: "goodbye world",
            "something": "cool {blah}",
            "something2": "cooler",
            div {
                div {
                    h1 { class: "h1 col" }
                    h2 { class: "h2 col" }
                    h3 { class: "h3 col" }
                    div {}
                }
            }
        }
    };

    let parsed: Element = syn::parse2(input).unwrap();
    println!("{}", parsed.to_token_stream().pretty_unparse());
}

#[test]
fn to_tokens_with_diagnostic() {
    let input = quote::quote! {
        div {
            class: "hello world",
            id: "my-id",
            ..attrs,
            div {
                ..attrs,
                class: "hello world",
                id: "my-id",
            }
        }
    };

    let parsed: Element = syn::parse2(input).unwrap();
    println!("{}", parsed.to_token_stream().pretty_unparse());
}

#[test]
fn merges_attributes() {
    let input = quote::quote! {
        div {
            class: "hello world",
            class: if count > 3 { "abc {def}" }
        }
    };

    let parsed: Element = syn::parse2(input).unwrap();
    assert_eq!(parsed.merged_attributes.len(), 1);
    assert_eq!(
        parsed.merged_attributes[0].name.to_string(),
        "class".to_string()
    );

    let attr = &parsed.merged_attributes[0].value;

    println!("{}", attr.to_token_stream().pretty_unparse());

    let _attr = match attr {
        AttributeValue::AttrLiteral(lit) => lit,
        _ => panic!("expected literal"),
    };
}

/// There are a number of cases where merging attributes doesn't make sense
/// - merging two expressions together
/// - merging two literals together
/// - merging a literal and an expression together
///
/// etc
///
/// We really only want to merge formatted things together
///
/// IE
/// class: "hello world ",
/// class: if some_expr { "abc" }
///
/// Some open questions - should the delimiter be explicit?
#[test]
fn merging_weird_fails() {
    let input = quote::quote! {
        div {
            class: "hello world",
            class: if some_expr { 123 },

            style: "color: red;",
            style: "color: blue;",

            width: "1px",
            width: 1,
            width: false,
            contenteditable: true,
        }
    };

    let parsed: Element = syn::parse2(input).unwrap();

    assert_eq!(parsed.merged_attributes.len(), 4);
    assert_eq!(parsed.diagnostics.len(), 3);

    // style should not generate a diagnostic
    assert!(!parsed
        .diagnostics
        .diagnostics
        .into_iter()
        .any(|f| f.emit_as_item_tokens().to_string().contains("style")));
}

#[test]
fn diagnostics() {
    let input = quote::quote! {
        p {
            class: "foo bar"
            "Hello world"
        }
    };

    let _parsed: Element = syn::parse2(input).unwrap();
}

#[test]
fn parses_raw_elements() {
    let input = quote::quote! {
        use {
            "hello"
        }
    };

    let _parsed: Element = syn::parse2(input).unwrap();
}
